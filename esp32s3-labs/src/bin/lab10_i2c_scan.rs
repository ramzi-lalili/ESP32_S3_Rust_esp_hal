//! Lab 10 - I2C master: bus scan
//!
//! Wiring (the LCD1602 backpack that already sits on the board):
//!   SDA -> GPIO41, SCL -> GPIO40, GND common. Use a 3.3 V-safe bus (see manual).
//! The scan sends an address byte with an empty write; a device that exists
//! pulls SDA low (ACK) and the driver returns Ok(()).
//! API level: STABLE (`esp_hal::i2c::master`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config, I2c},
    main,
    time::{Duration, Instant, Rate},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut i2c = I2c::new(
        peripherals.I2C0,
        Config::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap() // Err only if the configuration is invalid
    .with_sda(peripherals.GPIO41)
    .with_scl(peripherals.GPIO40);

    loop {
        info!("Scanning 7-bit addresses 0x08..=0x77 ...");
        let mut found = 0;
        for addr in 0x08u8..=0x77 {
            if i2c.write(addr, &[]).is_ok() {
                info!("  device found at 0x{:02X}", addr);
                found += 1;
            }
        }
        info!("scan complete: {} device(s)", found);

        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(3) {}
    }
}
