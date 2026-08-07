


#[derive(Debug, Clone, Copy)]
pub enum UiInbound {
    IsOn(bool),
    Brightness(u8),
    Online(bool),
}

#[derive(Debug, Clone, Copy)]
pub enum UiOutbound {
    SetIsOn(bool),
    SetBrightness(u8),
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    info!("checkpoint 1: boot");

    std::thread::Builder::new()
        .stack_size(32 * 1024)
        .spawn(app_main)?
        .join()
        .map_err(|_| anyhow::anyhow!("app_main panicked"))?
}

fn app_main() -> anyhow::Result<()> {
    let (peripherals, sysloop, nvs) = sys_start()?;
    let wifi = start_wifi(peripherals.modem, sysloop, nvs)?;
    start_display(peripherals);
    let display = start_touch(peripherals);
    let (window, ui) = start_slint_plattform(display);
    let (client, conn, in_rx, in_tx, out_rx, out_tx) = start_mqtt(ui);
    start_loop(window, client);
}

fn sys_start() -> anyhow::Result<(Peripherals, EspSystemEventLoop, EspDefaultNvsPartition)> {
    let peripherals = match Peripherals::take() {
        Ok(value) => value,
        Err(e) => return Ok(())
    };
    let sysloop = match EspSystemEventLoop::take() {
        Ok(value) => value,
        Err(error) => return Ok(())
    };
    let nvs = match EspDefaultNvsPartition::take() {
        Ok(value) => value,
        Err(error) => return Ok(())
    };
    Ok((peripherals, sysloop, nvs))
}

fn start_wifi(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> anyhow::Result<Box<EspWifi<'static>>> {
    let mut wifi = match make_wifi(modem, sysloop, nvs) {
        Ok(value) => value,
        Err(error) => return Err(anyhow::anyhow!("make_wifi failed: {:?}", error)),
    };

    if let Err(error) = connect_wifi(&mut wifi) {
        return Err(anyhow::anyhow!("connect_wifi failed: {:?}", error));
    }

    Ok(wifi)
}

fn start_display(
    peripherals: peripherals
) -> anyhow::Result<St7789<SpiDeviceDriver<'static, &'static SpiDriver<'static>>, PinDriver<'static, Gpio41, Output>, PinDriver<'static, Gpio39, Output>>> {
    let mut delay = Delay::new_default();
    let dc_pin = PinDriver::output(peripherals.pins.gpio41).context("DC pin")?;
    let rst_pin = PinDriver::output(peripherals.pins.gpio39).context("RST pin")?;
    let mut bl = PinDriver::output(peripherals.pins.gpio5).context("BL pin")?;
    bl.set_high().context("backlight enable failed")?;
    let spi_driver = SpiDriver::new(peripherals.spi2, peripherals.pins.gpio40, peripherals.pins.gpio45, Some(peripherals.pins.gpio46), &SpiDriverConfig::new(),).context("SpiDriver::new failed")?;
    let spi_driver: &'static SpiDriver<'static> = Box::leak(Box::new(spi_driver));
    let spi_device = SpiDeviceDriver::new(spi_driver, Some(peripherals.pins.gpio42), &SpiConfig::new().baudrate(40.MHz().into()),).context("SpiDeviceDriver::new failed")?;
    let display = St7789::new(spi_device, dc_pin, rst_pin, &mut delay).map_err(|_| anyhow::anyhow!("ST7789 init failed"))?;
    Ok(display)
}

fn start_touch(peripherals: peripherals) -> anyhow::Result<()> {
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio1,  // SDA
        peripherals.pins.gpio3,  // SCL
        &I2cConfig::new().baudrate(400.kHz().into()),
    ).context("I2cDriver::new failed")?;
    let mut touch = Cst328::new(i2c);
    Ok(touch)
}

fn start_slint_plattform(display: display) -> anyhow::Result<()>{
    let platform = St7789Platform::new(display);
    let window = platform.window();

    slint::platform::set_platform(Box::new(platform)).expect("Slint platform already set");

    let ui = AppWindow::new().expect("Slint UI init failed");
    ui.show().expect("Slint show failed");
    Ok(window, ui)
}

fn start_mqtt(ui: ui) -> anyhow::Result<()> {
    let (in_tx,  in_rx)  = mpsc::channel::<UiInbound>();
    let (out_tx, out_rx) = mpsc::channel::<UiOutbound>();
    {
        let tx = out_tx.clone();
        ui.on_publish_ison(move |v| { let _ = tx.send(UiOutbound::SetIsOn(v)); });
    }
    {
        let tx = out_tx.clone();
        ui.on_publish_brightness(move |value| {
            let brightness = value.clamp(1, 100) as u8;
            let _ = tx.send(UiOutbound::SetBrightness(brightness));
        });
    }
    let (mut client, mut conn) = make_mqtt().context("make_mqtt failed")?;

    std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || mqtt_worker(&mut conn, in_tx))?;

    Ok(client, conn, in_rx, in_tx, out_rx, out_tx)
}

fn start_loop(window: window, client: client) {
    let slint_window = window.window();
    let mut touch_active = false;
    let mut last_pos     = LogicalPosition::new(0.0_f32, 0.0_f32);
    let mut subscribed   = false;

    loop {
        if !subscribed {
            let ok1 = client.subscribe(config::TOPIC_ISON,       QoS::AtLeastOnce).is_ok();
            let ok2 = client.subscribe(config::TOPIC_BRIGHTNESS, QoS::AtLeastOnce).is_ok();
            if ok1 && ok2 {
                info!("subscribed to command topics");
                let _ = client.enqueue(
                    config::TOPIC_STATE, QoS::AtLeastOnce, true, b"online",
                );
                subscribed = true;
            }
        }
        loop {
            match out_rx.try_recv() {
                Ok(cmd) => { let _ = publish_outbound(&mut client, cmd); }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        while let Ok(msg) = in_rx.try_recv() {
            apply_inbound(&ui, msg);
        }
        slint::platform::update_timers_and_animations();
        process_touch(
            &mut touch,
            &slint_window,
            &mut touch_active,
            &mut last_pos,
        );
        window.render_frame().expect("render failed");
        std::thread::sleep(Duration::from_millis(16));
    }
}

fn handle_inbound(topic: Option<&str>, data: &[u8]) -> Option<UiInbound> {
    let topic = topic?;

    if topic == config::TOPIC_ISON {
        let s = core::str::from_utf8(data).ok()?.trim();
        let v = s.eq_ignore_ascii_case("true") || s == "1";
        return Some(UiInbound::IsOn(v));
    }

    if topic == config::TOPIC_BRIGHTNESS {
        let text = core::str::from_utf8(data).ok()?.trim();
        let value = text.parse::<u8>().ok()?;

        if (1..=100).contains(&value) {
            return Some(UiInbound::Brightness(value));
        }
    }

    None
}

fn apply_inbound(ui: &AppWindow, msg: UiInbound) {
    match msg {
        UiInbound::IsOn(v)      => ui.set_ison(v),
        UiInbound::Brightness(v) => ui.set_brightness(v as f32),
        UiInbound::Online(v)    => ui.set_connection(v),
    }
}

fn process_touch<I2C>(
    touch:        &mut Cst328<I2C>,
    slint_window: &slint::Window,
    active:       &mut bool,
    last_pos:     &mut LogicalPosition,
) where I2C: embedded_hal::i2c::I2c, {
    match touch.read_touch() {
        Ok((points, count)) if count > 0 => {
            let p   = points[0];
            let pos = LogicalPosition::new(p.x as f32, p.y as f32);
            *last_pos = pos;
            if !*active {
                slint_window.dispatch_event(WindowEvent::PointerPressed {
                    position: pos,
                    button:   PointerEventButton::Left,
                });
                *active = true;
            } else {
                slint_window.dispatch_event(WindowEvent::PointerMoved { position: pos });
            }
        }
        Ok(_) => {
            if *active {
                slint_window.dispatch_event(WindowEvent::PointerReleased {
                    position: *last_pos,
                    button:   PointerEventButton::Left,
                });
                *active = false;
            }
        }
        Err(_) => {}
    }
}