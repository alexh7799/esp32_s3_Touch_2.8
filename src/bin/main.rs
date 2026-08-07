use anyhow::Context;
use log::info;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

use esp_idf_hal::{
    delay::Delay,
    gpio::{Gpio39, Gpio40, Gpio41, Gpio42, Gpio45, Gpio46, Gpio5, Output, PinDriver},
    i2c::{I2cConfig, I2cDriver, I2C0},
    peripherals::Peripherals,
    spi::{SpiConfig, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2},
    units::FromValueType,
};

use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    mqtt::client::{EspMqttClient, QoS},
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};

use slint::platform::{PointerEventButton, WindowAdapter, WindowEvent};
use slint::LogicalPosition;

use esp32_wave_28::cst328::Cst328;
use esp32_wave_28::mqtt::{make_mqtt, mqtt_worker, publish_outbound};
use esp32_wave_28::slint_backend::{SlintWindow, St7789Platform};
use esp32_wave_28::st7789::St7789;
use esp32_wave_28::wifi::{connect_wifi, make_wifi};
use esp32_wave_28::{config, UiInbound, UiOutbound};

slint::include_modules!();

type DisplayDc = PinDriver<'static, Gpio41, Output>;
type DisplayRst = PinDriver<'static, Gpio39, Output>;
type DisplaySpi = SpiDeviceDriver<'static, &'static SpiDriver<'static>>;
type Display = St7789<DisplaySpi, DisplayDc, DisplayRst>;
type DisplayBl = PinDriver<'static, Gpio5, Output>;
type Touch = Cst328<I2cDriver<'static>>;
type UiWindow = Rc<SlintWindow<DisplaySpi, DisplayDc, DisplayRst>>;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    std::thread::Builder::new()
        .stack_size(32 * 1024)
        .spawn(app_main)?
        .join()
        .map_err(|_| anyhow::anyhow!("app_main panicked"))?
}

fn app_main() -> anyhow::Result<()> {
    let (p, sysloop, nvs) = sys_start()?;
    let modem = p.modem;
    let (display, _backlight) = start_display(
        p.spi2,
        p.pins.gpio40,
        p.pins.gpio45,
        p.pins.gpio46,
        p.pins.gpio42,
        p.pins.gpio41,
        p.pins.gpio39,
        p.pins.gpio5,
    )?;
    let touch = start_touch(p.i2c0, p.pins.gpio1, p.pins.gpio3)?;
    let _wifi = start_wifi(modem, sysloop, nvs)?;
    let (window, ui) = start_slint_platform(display)?;
    let (client, in_rx, out_rx) = start_mqtt(&ui)?;
    start_loop(window, client, ui, touch, in_rx, out_rx)
}

fn sys_start() -> anyhow::Result<(Peripherals, EspSystemEventLoop, EspDefaultNvsPartition)> {
    let peripherals = Peripherals::take().context("Peripherals::take failed")?;
    let sysloop = EspSystemEventLoop::take().context("EspSystemEventLoop::take failed")?;
    let nvs = EspDefaultNvsPartition::take().context("EspDefaultNvsPartition::take failed")?;
    Ok((peripherals, sysloop, nvs))
}

fn start_wifi(
    modem: esp_idf_hal::modem::Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let mut wifi = make_wifi(modem, sysloop, nvs)?;
    connect_wifi(&mut wifi)?;
    Ok(wifi)
}

fn start_display(
    spi: SPI2,
    sck: Gpio40,
    mosi: Gpio45,
    miso: Gpio46,
    cs: Gpio42,
    dc: Gpio41,
    rst: Gpio39,
    bl_pin: Gpio5,
) -> anyhow::Result<(Display, DisplayBl)> {
    let mut delay = Delay::new_default();
    let dc_pin = PinDriver::output(dc).context("DC pin")?;
    let rst_pin = PinDriver::output(rst).context("RST pin")?;
    let mut bl = PinDriver::output(bl_pin).context("BL pin")?;
    bl.set_high().context("backlight enable failed")?;
    let spi_driver = SpiDriver::new(spi, sck, mosi, Some(miso), &SpiDriverConfig::new())
        .context("SpiDriver::new failed")?;
    let spi_driver: &'static SpiDriver<'static> = Box::leak(Box::new(spi_driver));
    let spi_device = SpiDeviceDriver::new(
        spi_driver,
        Some(cs),
        &SpiConfig::new().baudrate(40.MHz().into()),
    )
    .context("SpiDeviceDriver::new failed")?;
    let display = St7789::new(spi_device, dc_pin, rst_pin, &mut delay)
        .map_err(|_| anyhow::anyhow!("ST7789 init failed"))?;
    Ok((display, bl))
}

fn start_touch(
    i2c: I2C0,
    sda: esp_idf_hal::gpio::Gpio1,
    scl: esp_idf_hal::gpio::Gpio3,
) -> anyhow::Result<Touch> {
    let i2c_driver = I2cDriver::new(i2c, sda, scl, &I2cConfig::new().baudrate(400.kHz().into()))
        .context("I2cDriver::new failed")?;
    Ok(Cst328::new(i2c_driver))
}

fn start_slint_platform(display: Display) -> anyhow::Result<(UiWindow, AppWindow)> {
    let platform = St7789Platform::new(display);
    let window = platform.window();
    slint::platform::set_platform(Box::new(platform))
        .map_err(|_| anyhow::anyhow!("Slint platform already set"))?;
    let ui = AppWindow::new().context("Slint UI init failed")?;
    ui.show().context("Slint show failed")?;
    Ok((window, ui))
}

fn start_mqtt(
    ui: &AppWindow,
) -> anyhow::Result<(
    EspMqttClient<'static>,
    mpsc::Receiver<UiInbound>,
    mpsc::Receiver<UiOutbound>,
)> {
    let (in_tx, in_rx) = mpsc::channel::<UiInbound>();
    let (out_tx, out_rx) = mpsc::channel::<UiOutbound>();
    {
        let tx = out_tx.clone();
        ui.on_publish_ison(move |v| {
            let _ = tx.send(UiOutbound::SetIsOn(v));
        });
    }
    {
        let tx = out_tx;
        ui.on_publish_brightness(move |value| {
            let brightness = value.clamp(1, 100) as u8;
            let _ = tx.send(UiOutbound::SetBrightness(brightness));
        });
    }
    let (client, mut conn) = make_mqtt()?;
    std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || mqtt_worker(&mut conn, in_tx))
        .context("MQTT worker thread spawn failed")?;
    Ok((client, in_rx, out_rx))
}

fn start_loop(
    window: UiWindow,
    mut client: EspMqttClient<'static>,
    ui: AppWindow,
    mut touch: Touch,
    in_rx: mpsc::Receiver<UiInbound>,
    out_rx: mpsc::Receiver<UiOutbound>,
) -> anyhow::Result<()> {
    let slint_window = window.window();
    let mut touch_active = false;
    let mut last_pos = LogicalPosition::new(0.0_f32, 0.0_f32);
    let mut subscribed = false;
    loop {
        if !subscribed {
            subscribed = try_subscribe(&mut client);
        }
        drain_outbound(&mut client, &out_rx);
        drain_inbound(&ui, &in_rx);
        slint::platform::update_timers_and_animations();
        process_touch(&mut touch, &slint_window, &mut touch_active, &mut last_pos);
        window.render_frame().expect("render failed");
        std::thread::sleep(Duration::from_millis(16));
    }
}

fn try_subscribe(client: &mut esp_idf_svc::mqtt::client::EspMqttClient<'static>) -> bool {
    let ok1 = client
        .subscribe(config::TOPIC_ISON, QoS::AtLeastOnce)
        .is_ok();
    let ok2 = client
        .subscribe(config::TOPIC_BRIGHTNESS, QoS::AtLeastOnce)
        .is_ok();
    if ok1 && ok2 {
        info!("subscribed to command topics");
        let _ = client.enqueue(config::TOPIC_STATE, QoS::AtLeastOnce, true, b"online");
        return true;
    }
    false
}

fn drain_outbound(
    client: &mut esp_idf_svc::mqtt::client::EspMqttClient<'static>,
    rx: &std::sync::mpsc::Receiver<UiOutbound>,
) {
    loop {
        match rx.try_recv() {
            Ok(cmd) => {
                let _ = publish_outbound(client, cmd);
            }
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
        }
    }
}

fn drain_inbound(ui: &AppWindow, rx: &std::sync::mpsc::Receiver<UiInbound>) {
    while let Ok(msg) = rx.try_recv() {
        apply_inbound(ui, msg);
    }
}

fn apply_inbound(ui: &AppWindow, msg: UiInbound) {
    match msg {
        UiInbound::IsOn(v) => ui.set_ison(v),
        UiInbound::Brightness(v) => ui.set_brightness(v as f32),
        UiInbound::Online(v) => ui.set_connection(v),
    }
}

fn process_touch<I2C>(
    touch: &mut Cst328<I2C>,
    slint_window: &slint::Window,
    active: &mut bool,
    last_pos: &mut LogicalPosition,
) where
    I2C: embedded_hal::i2c::I2c,
{
    match touch.read_touch() {
        Ok((points, count)) if count > 0 => {
            let p = points[0];
            let pos = LogicalPosition::new(p.x as f32, p.y as f32);
            *last_pos = pos;
            if !*active {
                slint_window.dispatch_event(WindowEvent::PointerPressed {
                    position: pos,
                    button: PointerEventButton::Left,
                });
                *active = true;
            } else {
                slint_window.dispatch_event(WindowEvent::PointerMoved { position: pos });
            }
        }
        Ok(_) if *active => {
            slint_window.dispatch_event(WindowEvent::PointerReleased {
                position: *last_pos,
                button: PointerEventButton::Left,
            });
            *active = false;
        }
        _ => {}
    }
}
