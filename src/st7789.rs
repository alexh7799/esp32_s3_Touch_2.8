use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiDevice};

pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 320;

#[allow(dead_code)]
mod cmd {
    pub const SWRESET: u8 = 0x01;
    pub const SLPOUT: u8 = 0x11;
    pub const COLMOD: u8 = 0x3A;
    pub const MADCTL: u8 = 0x36;
    pub const CASET: u8 = 0x2A;
    pub const RASET: u8 = 0x2B;
    pub const RAMWR: u8 = 0x2C;
    pub const DISPON: u8 = 0x29;
    pub const INVON: u8 = 0x21;
}

pub struct St7789<SPI, DC, RST> {
    spi: SPI,
    dc: DC,
    rst: RST,
}

impl<SPI, DC, RST> St7789<SPI, DC, RST>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
{
    pub fn new<D: DelayNs>(spi: SPI, dc: DC, rst: RST, delay: &mut D) -> Result<Self, InitError> {
        let mut driver = Self { spi, dc, rst };
        driver.hard_reset(delay).map_err(|_| InitError::Gpio)?;
        driver.init_sequence(delay).map_err(|_| InitError::Spi)?;
        Ok(driver)
    }

    pub fn flush_line(
        &mut self,
        line: u16,
        range: &core::ops::Range<usize>,
        bytes: &[u8],
    ) -> Result<(), DrawError> {
        let x0 = range.start as u16;
        let x1 = (range.end as u16).saturating_sub(1);
        self.set_window(x0, line, x1, line)
            .map_err(|_| DrawError::Spi)?;
        self.write_data(bytes).map_err(|_| DrawError::Spi)
    }

    fn hard_reset<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), RST::Error> {
        self.rst.set_high()?;
        delay.delay_ms(10);
        self.rst.set_low()?;
        delay.delay_ms(10);
        self.rst.set_high()?;
        delay.delay_ms(120);
        Ok(())
    }

    fn init_sequence<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), SPI::Error> {
        self.write_cmd(cmd::SWRESET)?;
        delay.delay_ms(150);
        self.write_cmd(cmd::SLPOUT)?;
        delay.delay_ms(10);
        self.write_cmd_data(cmd::COLMOD, &[0x55])?;
        self.write_cmd_data(cmd::MADCTL, &[0x00])?;
        self.write_cmd(cmd::INVON)?;
        self.write_cmd(cmd::DISPON)?;
        delay.delay_ms(10);
        Ok(())
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), SPI::Error> {
        self.write_cmd_data(cmd::CASET, &u16_pair_bytes(x0, x1))?;
        self.write_cmd_data(cmd::RASET, &u16_pair_bytes(y0, y1))?;
        self.write_cmd(cmd::RAMWR)
    }

    fn write_cmd(&mut self, cmd: u8) -> Result<(), SPI::Error> {
        self.dc.set_low().ok();
        self.spi.write(&[cmd])
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), SPI::Error> {
        self.dc.set_high().ok();
        self.spi.write(data)
    }

    fn write_cmd_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), SPI::Error> {
        self.write_cmd(cmd)?;
        self.write_data(data)
    }
}

#[derive(Debug)]
pub enum InitError {
    Spi,
    Gpio,
}

#[derive(Debug)]
pub enum DrawError {
    Spi,
}

#[inline]
fn u16_pair_bytes(a: u16, b: u16) -> [u8; 4] {
    [(a >> 8) as u8, a as u8, (b >> 8) as u8, b as u8]
}
