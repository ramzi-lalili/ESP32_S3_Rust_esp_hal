#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    delay::Delay,
    main,
    spi::{
        master::{Config, Spi},
        Mode,
    },
    time::Rate,
};

// Loopback test: wire GPIO48 (MOSI) to GPIO42 (MISO) with a jumper.
#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut spi = Spi::new(
        p.SPI2,
        Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(p.GPIO47)
    .with_mosi(p.GPIO48)
    .with_miso(p.GPIO42);
    let delay = Delay::new();

    loop {
        let mut buf = [0xDE, 0xAD, 0xBE, 0xEF];
        spi.transfer(&mut buf).unwrap(); // full duplex: tx bytes are replaced by rx bytes
        println!("loopback returned {:02X?}", buf);
        delay.delay_millis(1000);
    }
}
