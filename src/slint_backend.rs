
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Instant as StdInstant;

use slint::platform::{
    software_renderer::{
        LineBufferProvider, RepaintBufferType, Rgb565Pixel, SoftwareRenderer,
    },
    Platform, WindowAdapter,
};
use slint::{PhysicalSize, Window};

use embedded_hal::{digital::OutputPin, spi::SpiDevice};

use crate::st7789::{DrawError, St7789, HEIGHT, WIDTH};


const LINE_PIXELS: usize = WIDTH as usize;

pub struct SlintWindow<SPI, DC, RST> {
    window:   Window,
    renderer: SoftwareRenderer,
    display:  RefCell<St7789<SPI, DC, RST>>,
}

impl<SPI, DC, RST> SlintWindow<SPI, DC, RST>
where
    SPI: SpiDevice + 'static,
    DC:  OutputPin + 'static,
    RST: OutputPin + 'static,
{
    fn new(display: St7789<SPI, DC, RST>) -> Rc<Self> {
        let renderer = SoftwareRenderer::new_with_repaint_buffer_type(
            RepaintBufferType::ReusedBuffer,
        );
        Rc::new_cyclic(|weak| {
            let window = Window::new(weak.clone() as _);
            SlintWindow {
                window,
                renderer,
                display: RefCell::new(display),
            }
        })
    }

    pub fn render_frame(&self) -> Result<(), DrawError> {
        let mut line_buf = [Rgb565Pixel(0u16); LINE_PIXELS];
        self.renderer.render_by_line(LineDrawer {
            display:  &self.display,
            line_buf: &mut line_buf,
        });
        Ok(())
    }
}

impl<SPI, DC, RST> WindowAdapter for SlintWindow<SPI, DC, RST>
where
    SPI: SpiDevice + 'static,
    DC:  OutputPin + 'static,
    RST: OutputPin + 'static,
{
    fn window(&self) -> &slint::Window {
        &self.window
    }

    fn renderer(&self) -> &dyn slint::platform::Renderer {
        &self.renderer
    }

    fn size(&self) -> PhysicalSize {
        PhysicalSize::new(WIDTH as u32, HEIGHT as u32)
    }
}

pub struct St7789Platform<SPI, DC, RST> {
    window:     Rc<SlintWindow<SPI, DC, RST>>,
    boot_time:  StdInstant,
}

impl<SPI, DC, RST> St7789Platform<SPI, DC, RST>
where
    SPI: SpiDevice + 'static,
    DC:  OutputPin + 'static,
    RST: OutputPin + 'static,
{
    pub fn new(display: St7789<SPI, DC, RST>) -> Self {
        Self {
            window:    SlintWindow::new(display),
            boot_time: StdInstant::now(),
        }
    }

    pub fn window(&self) -> Rc<SlintWindow<SPI, DC, RST>> {
        self.window.clone()
    }
}

impl<SPI, DC, RST> Platform for St7789Platform<SPI, DC, RST>
where
    SPI: SpiDevice + 'static,
    DC:  OutputPin + 'static,
    RST: OutputPin + 'static,
{
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        Ok(())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        self.boot_time.elapsed()
    }
}

fn encode_rgb565(pixels: &[Rgb565Pixel], bytes: &mut [u8]) {
    for (pixel, out) in pixels.iter().zip(bytes.chunks_exact_mut(2)) {
        out[0] = (pixel.0 >> 8) as u8;
        out[1] = pixel.0 as u8;
    }
}

struct LineDrawer<'a, SPI, DC, RST> {
    display:  &'a RefCell<St7789<SPI, DC, RST>>,
    line_buf: &'a mut [Rgb565Pixel; LINE_PIXELS],
}

impl<SPI, DC, RST> LineBufferProvider for LineDrawer<'_, SPI, DC, RST>
where
    SPI: SpiDevice,
    DC:  OutputPin,
    RST: OutputPin,
{
    type TargetPixel = Rgb565Pixel;

    fn process_line(
        &mut self,
        line:      usize,
        range:     core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [Rgb565Pixel]),
    ) {
        let buf = &mut self.line_buf[..range.len()];
        render_fn(buf);
        let mut bytes = [0u8; LINE_PIXELS * 2];
        encode_rgb565(buf, &mut bytes);
        let _ = self.display.borrow_mut().flush_line(
            line as u16,
            &range,
            &bytes[..range.len() * 2],
        );
    }
}
