#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use core::fmt::Write;
use esp32s3_lab::lcd1602::Lcd;
use esp_hal::{
    delay::Delay,
    i2c::master::{Config, I2c},
    main,
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let i2c = I2c::new(p.I2C0, Config::default())
        .unwrap()
        .with_sda(p.GPIO41)
        .with_scl(p.GPIO40);

    let mut lcd = Lcd::new(i2c, 0x27);
    lcd.init();
    lcd.print("ESP32-S3 Rust");

    let delay = Delay::new();
    let mut n: u32 = 0;
    loop {
        lcd.set_cursor(0, 1);
        write!(lcd, "count = {:6}", n).ok();
        n += 1;
        delay.delay_millis(500);
    }
}
