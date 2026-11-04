//! Minimal HD44780 (LCD1602) driver for the common PCF8574 I²C backpack.
//!
//! PCF8574 → HD44780 wiring used by most backpacks (YwRobot / DFRobot style):
//!
//! ```text
//!  PCF8574 bit :  P7  P6  P5  P4  P3  P2  P1  P0
//!  LCD signal  :  D7  D6  D5  D4  BL  EN  RW  RS
//! ```
//!
//! The display is driven in 4-bit mode: each byte is sent as two nibbles, and
//! every nibble is latched with one pulse on EN. The driver is generic over the
//! `embedded-hal` 1.0 [`I2c`] and [`DelayNs`] traits, so it works with the
//! blocking esp-hal `I2c` driver and with `esp_hal::delay::Delay`.
//!
//! STATUS: build-verified only. It follows the standard HD44780 initialisation
//! sequence but has NOT been tested on real hardware by the author.
use core::fmt;
use embedded_hal::{delay::DelayNs, i2c::I2c};

const RS: u8 = 0x01;
const EN: u8 = 0x04;
const BACKLIGHT: u8 = 0x08;

/// HD44780 instruction codes used by this driver.
mod cmd {
    pub const CLEAR: u8 = 0x01;
    pub const HOME: u8 = 0x02;
    pub const ENTRY_MODE_INC: u8 = 0x06;
    pub const DISPLAY_ON: u8 = 0x0C; // display on, cursor off, blink off
    pub const FUNCTION_4BIT_2LINE: u8 = 0x28;
    pub const SET_DDRAM: u8 = 0x80;
}

/// LCD1602 behind a PCF8574 I²C expander.
pub struct Lcd1602<I2C, D> {
    i2c: I2C,
    delay: D,
    addr: u8,
    backlight: u8,
}

impl<I2C: I2c, D: DelayNs> Lcd1602<I2C, D> {
    /// Create the driver. Nothing is sent to the display until [`Self::init`].
    pub fn new(i2c: I2C, delay: D, addr: u8) -> Self {
        Self { i2c, delay, addr, backlight: BACKLIGHT }
    }

    /// Run the HD44780 power-on sequence for 4-bit mode (datasheet fig. 24).
    pub fn init(&mut self) -> Result<(), I2C::Error> {
        self.delay.delay_ms(50); // > 40 ms after Vcc reaches 2.7 V
        self.write_nibble(0x30, false)?; // 8-bit "function set", sent three
        self.delay.delay_ms(5); //          times to force a known state
        self.write_nibble(0x30, false)?;
        self.delay.delay_us(200);
        self.write_nibble(0x30, false)?;
        self.delay.delay_us(200);
        self.write_nibble(0x20, false)?; // switch to 4-bit interface
        self.delay.delay_us(200);
        self.command(cmd::FUNCTION_4BIT_2LINE)?;
        self.command(cmd::DISPLAY_ON)?;
        self.command(cmd::ENTRY_MODE_INC)?;
        self.clear()
    }

    /// Clear the display and return the cursor to (0, 0).
    pub fn clear(&mut self) -> Result<(), I2C::Error> {
        self.command(cmd::CLEAR)?;
        self.delay.delay_ms(2); // CLEAR needs ~1.52 ms
        Ok(())
    }

    /// Return the cursor to (0, 0) without clearing.
    pub fn home(&mut self) -> Result<(), I2C::Error> {
        self.command(cmd::HOME)?;
        self.delay.delay_ms(2);
        Ok(())
    }

    /// Move the cursor. `row` is 0 or 1, `col` is 0..=15.
    pub fn set_cursor(&mut self, col: u8, row: u8) -> Result<(), I2C::Error> {
        let offset = if row == 0 { 0x00 } else { 0x40 };
        self.command(cmd::SET_DDRAM | (offset + col.min(15)))
    }

    /// Turn the backlight on or off (takes effect on the next write).
    pub fn set_backlight(&mut self, on: bool) -> Result<(), I2C::Error> {
        self.backlight = if on { BACKLIGHT } else { 0 };
        self.expander_write(0)
    }

    /// Print an ASCII string at the current cursor position.
    pub fn print(&mut self, text: &str) -> Result<(), I2C::Error> {
        for b in text.bytes() {
            self.data(b)?;
        }
        Ok(())
    }

    /// Send an instruction byte (RS = 0).
    pub fn command(&mut self, value: u8) -> Result<(), I2C::Error> {
        self.send(value, false)
    }

    /// Send a data byte (RS = 1), i.e. one character.
    pub fn data(&mut self, value: u8) -> Result<(), I2C::Error> {
        self.send(value, true)
    }

    fn send(&mut self, value: u8, rs: bool) -> Result<(), I2C::Error> {
        self.write_nibble(value & 0xF0, rs)?; // high nibble first
        self.write_nibble((value << 4) & 0xF0, rs)?;
        self.delay.delay_us(50); // most instructions take < 40 µs
        Ok(())
    }

    /// Put one nibble (already in bits 7..4) on D7..D4 and pulse EN.
    fn write_nibble(&mut self, nibble: u8, rs: bool) -> Result<(), I2C::Error> {
        let base = nibble | if rs { RS } else { 0 };
        self.expander_write(base | EN)?; // EN high: LCD samples on falling edge
        self.delay.delay_us(1);
        self.expander_write(base)?; // EN low
        self.delay.delay_us(50);
        Ok(())
    }

    fn expander_write(&mut self, bits: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.addr, &[bits | self.backlight])
    }
}

/// Allows `write!(lcd, "T={:.1}C", t)`. I²C errors are mapped to `fmt::Error`.
impl<I2C: I2c, D: DelayNs> fmt::Write for Lcd1602<I2C, D> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print(s).map_err(|_| fmt::Error)
    }
}
