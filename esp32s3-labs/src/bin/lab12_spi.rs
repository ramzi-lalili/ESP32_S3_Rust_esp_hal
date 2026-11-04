//! Lab 12 - SPI master (full-duplex loopback)
//!
//! Wiring (SPI2 / "FSPI" on the GPIO matrix):
//!   SCK  = GPIO39      MOSI = GPIO38      MISO = GPIO42      CS = GPIO21 (software-driven)
//!   Jumper: GPIO38 (MOSI)  <-->  GPIO42 (MISO)   => whatever is sent comes back.
//! API level: STABLE only (`esp_hal::spi::master` + `gpio`). The hardware chip-select helper
//! `Spi::with_cs` is marked #[instability::unstable] in 1.1.1, so this lab drives CS with a
//! plain `Output` pin, which also works with any number of devices on one bus.
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    main,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::{Duration, Instant, Rate},
};
use log::{error, info};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(1))
            .with_mode(Mode::_0), // CPOL=0, CPHA=0
    )
    .unwrap()
    .with_sck(peripherals.GPIO39)
    .with_mosi(peripherals.GPIO38)
    .with_miso(peripherals.GPIO42);

    // Chip select is an ordinary output: idle high, low while a transfer is in progress.
    let mut cs = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());

    let mut round: u8 = 0;
    loop {
        // `transfer` is full duplex and works in place: the buffer holds the
        // bytes to send on entry and the bytes received on exit.
        let mut buf = [0xA5, round, 0x3C, 0xFF, 0x00];
        let sent = buf;
        cs.set_low();
        spi.transfer(&mut buf).unwrap();
        cs.set_high();

        if buf == sent {
            info!("SPI loopback OK: {:02X?}", buf);
        } else {
            error!("SPI loopback FAILED: sent {:02X?} got {:02X?}", sent, buf);
        }
        round = round.wrapping_add(1);

        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(500) {}
    }
}
