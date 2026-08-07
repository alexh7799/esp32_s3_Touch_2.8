use esp_idf_hal::delay::Delay;
use esp_idf_hal::gpio::{Gpio15, InputOutput, PinDriver};

/// Raw sensor reading from the DHT11.
#[derive(Debug, Clone, Copy)]
pub struct Dht11Reading {
    pub temperature: u8,
    pub humidity: u8,
}

pub fn read_dht11(
    pin: &mut PinDriver<'static, Gpio15, InputOutput>,
    delay: &mut Delay,
) -> Result<Dht11Reading, &'static str> {
    send_start(pin, delay)?;
    wait_response(pin, delay)?;
    let bytes = read_bytes(pin, delay)?;
    validate(bytes)
}

fn send_start(
    pin: &mut PinDriver<'static, Gpio15, InputOutput>,
    delay: &mut Delay,
) -> Result<(), &'static str> {
    pin.set_low().map_err(|_| "set_low failed")?;
    delay.delay_ms(18);
    pin.set_high().map_err(|_| "set_high failed")?;
    delay.delay_us(40);
    Ok(())
}

fn wait_response(
    pin: &mut PinDriver<'static, Gpio15, InputOutput>,
    delay: &mut Delay,
) -> Result<(), &'static str> {
    wait_level(pin, delay, false, 200)?;
    wait_level(pin, delay, true, 200)?;
    wait_level(pin, delay, false, 200)
}

fn read_bytes(
    pin: &mut PinDriver<'static, Gpio15, InputOutput>,
    delay: &mut Delay,
) -> Result<[u8; 5], &'static str> {
    let mut bytes = [0u8; 5];
    for byte in &mut bytes {
        *byte = read_byte(pin, delay)?;
    }
    Ok(bytes)
}

fn read_byte(
    pin: &mut PinDriver<'static, Gpio15, InputOutput>,
    delay: &mut Delay,
) -> Result<u8, &'static str> {
    let mut byte = 0u8;
    for _ in 0..8 {
        byte <<= 1;
        byte |= read_bit(pin, delay)?;
    }
    Ok(byte)
}

fn read_bit(
    pin: &mut PinDriver<'static, Gpio15, InputOutput>,
    delay: &mut Delay,
) -> Result<u8, &'static str> {
    wait_level(pin, delay, true, 100)?;
    delay.delay_us(40_u32);
    let bit = pin.is_high() as u8;
    wait_level(pin, delay, false, 100)?;
    Ok(bit)
}

fn wait_level(
    pin: &mut PinDriver<'static, Gpio15, InputOutput>,
    delay: &mut Delay,
    high: bool,
    timeout_us: u32,
) -> Result<(), &'static str> {
    for _ in 0..timeout_us {
        if pin.is_high() == high {
            return Ok(());
        }
        delay.delay_us(1_u32);
    }
    Err("DHT11 timeout")
}

fn validate(bytes: [u8; 5]) -> Result<Dht11Reading, &'static str> {
    let expected = bytes[0]
        .wrapping_add(bytes[1])
        .wrapping_add(bytes[2])
        .wrapping_add(bytes[3]);
    if bytes[4] != expected {
        return Err("DHT11 checksum mismatch");
    }
    Ok(Dht11Reading {
        humidity: bytes[0],
        temperature: bytes[2],
    })
}
