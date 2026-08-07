use embedded_hal::i2c::I2c;

pub const DEFAULT_ADDR: u8 = 0x1A;

const REG_TOUCH_INFO: u16 = 0xD000;
const BYTES_PER_FRAME: usize = 27;
const BYTES_PER_POINT: usize = 5;
const MAX_POINTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
    pub pressure: u8,
}

pub struct Cst328<I2C> {
    i2c: I2C,
    addr: u8,
}

impl<I2C: I2c> Cst328<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: DEFAULT_ADDR,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_addr(i2c: I2C, addr: u8) -> Self {
        Self { i2c, addr }
    }

    pub fn read_touch(&mut self) -> Result<([TouchPoint; MAX_POINTS], usize), I2C::Error> {
        let mut frame = [0u8; BYTES_PER_FRAME];
        self.read_register(REG_TOUCH_INFO, &mut frame)?;
        let mut points = [empty_point(); MAX_POINTS];
        let mut count = 0;
        for (index, point) in points.iter_mut().enumerate() {
            let offset = point_offset(index);
            if is_active(frame[offset]) {
                *point = decode_point(&frame[offset..offset + BYTES_PER_POINT]);
                count += 1;
            }
        }
        Ok((points, count))
    }

    fn read_register(&mut self, register: u16, data: &mut [u8]) -> Result<(), I2C::Error> {
        self.i2c
            .write_read(self.addr, &register.to_be_bytes(), data)
    }
}

fn point_offset(index: usize) -> usize {
    if index == 0 {
        0
    } else {
        7 + (index - 1) * BYTES_PER_POINT
    }
}

fn is_active(value: u8) -> bool {
    value & 0x0F == 6
}

fn decode_point(data: &[u8]) -> TouchPoint {
    let x = (u16::from(data[1]) << 4) | u16::from(data[3] >> 4);
    let y = (u16::from(data[2]) << 4) | u16::from(data[3] & 0x0F);
    TouchPoint {
        x,
        y,
        pressure: data[4],
    }
}

const fn empty_point() -> TouchPoint {
    TouchPoint {
        x: 0,
        y: 0,
        pressure: 0,
    }
}
