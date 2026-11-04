#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    delay::Delay,
    i2c::master::{Config, I2c},
    main,
};

#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    // LCD backpack bus: SDA = GPIO41, SCL = GPIO40.
    let mut i2c = I2c::new(p.I2C0, Config::default())
        .unwrap()
        .with_sda(p.GPIO41)
        .with_scl(p.GPIO40);
    let delay = Delay::new();

    loop {
        println!("scanning...");
        for addr in 0x08u8..=0x77 {
            // An empty write sends only START + address; an ACK means a device is present.
            if i2c.write(addr, &[]).is_ok() {
                println!("  device at 0x{:02X}", addr);
            }
        }
        delay.delay_millis(3000);
    }
}
