//! Minimal HD44780 LCD1602 driver for the PCF8574 I2C backpack (4-bit mode).
//! Backpack bit map (most common): P0=RS P1=RW P2=EN P3=BACKLIGHT P4..P7=D4..D7.
use core::fmt;
use esp_hal::{delay::Delay, i2c::master::I2c, Blocking};

const RS: u8 = 0x01;
const EN: u8 = 0x04;
const BL: u8 = 0x08;

pub struct Lcd<'d> {
    i2c: I2c<'d, Blocking>,
    addr: u8,
    delay: Delay,
    backlight: u8,
}

impl<'d> Lcd<'d> {
    pub fn new(i2c: I2c<'d, Blocking>, addr: u8) -> Self {
        Self { i2c, addr, delay: Delay::new(), backlight: BL }
    }

    fn expander(&mut self, byte: u8) {
        // Errors are ignored on purpose: a missing display must not crash the firmware.
        let _ = self.i2c.write(self.addr, &[byte | self.backlight]);
    }

    fn pulse(&mut self, byte: u8) {
        self.expander(byte | EN);
        self.delay.delay_micros(1);
        self.expander(byte & !EN);
        self.delay.delay_micros(50);
    }

    fn write4(&mut self, nibble: u8, rs: u8) {
        self.pulse(((nibble & 0x0F) << 4) | rs);
    }

    fn send(&mut self, byte: u8, rs: u8) {
        self.write4(byte >> 4, rs);
        self.write4(byte & 0x0F, rs);
    }

    pub fn command(&mut self, cmd: u8) {
        self.send(cmd, 0);
    }

    pub fn init(&mut self) {
        self.delay.delay_millis(50);
        self.write4(0x3, 0);
        self.delay.delay_millis(5);
        self.write4(0x3, 0);
        self.delay.delay_micros(150);
        self.write4(0x3, 0);
        self.write4(0x2, 0); // switch to 4-bit mode
        self.command(0x28);  // 4-bit, 2 lines, 5x8 font
        self.command(0x0C);  // display on, cursor off
        self.command(0x06);  // entry mode: increment
        self.clear();
    }

    pub fn clear(&mut self) {
        self.command(0x01);
        self.delay.delay_millis(2);
    }

    pub fn set_cursor(&mut self, col: u8, row: u8) {
        let base = if row == 0 { 0x00 } else { 0x40 };
        self.command(0x80 | (base + col));
    }

    pub fn print(&mut self, s: &str) {
        for b in s.bytes() {
            self.send(b, RS);
        }
    }
}

impl fmt::Write for Lcd<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print(s);
        Ok(())
    }
}
