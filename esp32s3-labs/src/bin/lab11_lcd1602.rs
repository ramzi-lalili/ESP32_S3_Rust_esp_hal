//! Lab 11 - LCD1602 over I2C (PCF8574 backpack, address 0x27)
//!
//! Wiring: SDA -> GPIO41, SCL -> GPIO40, GND, VCC (see the 5 V warning in the manual:
//! the ESP32-S3 pins are NOT 5 V tolerant -> use a level shifter or a 3.3 V-safe backpack).
//! Uses the shared driver in `src/lcd1602.rs` (build-verified, not hardware-tested).
//! API level: STABLE I2C + UNSTABLE `esp_hal::delay::Delay`.
#![no_std]
#![no_main]

use core::fmt::Write;

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    i2c::master::{Config, I2c},
    main,
    time::Rate,
};
use esp32s3_labs::{lcd1602::Lcd1602, reserve_camera_pins};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

const LCD_ADDR: u8 = 0x27; // some backpacks use 0x3F

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _camera = reserve_camera_pins!(peripherals);

    let i2c = I2c::new(
        peripherals.I2C0,
        Config::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO41)
    .with_scl(peripherals.GPIO40);

    let mut lcd = Lcd1602::new(i2c, Delay::new(), LCD_ADDR);
    lcd.init().expect("LCD not answering on I2C - check wiring/address");

    lcd.set_cursor(0, 0).unwrap();
    lcd.print("ESP32-S3 + Rust").unwrap();

    let delay = Delay::new();
    let mut seconds: u32 = 0;
    loop {
        lcd.set_cursor(0, 1).unwrap();
        // `write!` works because Lcd1602 implements core::fmt::Write.
        write!(lcd, "uptime {:>6} s", seconds).unwrap();
        info!("uptime {} s", seconds);
        seconds += 1;
        delay.delay_millis(1000);
    }
}
