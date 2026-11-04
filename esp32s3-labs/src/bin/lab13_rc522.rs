//! Lab 13 - RC522 (MFRC522) RFID reader over SPI
//!
//! Wiring (RC522 module is a 3.3 V device - do NOT power it from 5 V):
//!   RC522 SDA (= chip select) -> GPIO21     RC522 SCK  -> GPIO39
//!   RC522 MOSI                -> GPIO38     RC522 MISO -> GPIO42
//!   RC522 RST                 -> GPIO14     RC522 3.3V -> 3V3,  GND -> GND
//!   RC522 IRQ                 -> not connected
//!
//! The SPI *bus* (`Spi`) implements embedded-hal's `SpiBus`. The `mfrc522` crate
//! needs a `SpiDevice` (bus + chip-select), which `embedded-hal-bus::ExclusiveDevice`
//! builds for us.
//! API level: STABLE SPI + UNSTABLE Delay. Driver crate: mfrc522 0.8.0 (build-verified only).
#![no_std]
#![no_main]

use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};
use log::{info, warn};
use mfrc522::{Mfrc522, comm::blocking::spi::SpiInterface};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // Hardware reset of the RC522: pull RST low, then release.
    let mut rst = Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default());
    delay.delay_millis(50);
    rst.set_high();
    delay.delay_millis(50);

    let spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO39)
    .with_mosi(peripherals.GPIO38)
    .with_miso(peripherals.GPIO42);

    // Chip select is driven in software by ExclusiveDevice.
    let cs = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());
    let spi_dev = ExclusiveDevice::new(spi_bus, cs, Delay::new()).unwrap();

    // The RC522 needs NSS high for >= 50 ns between transfers; in release
    // builds the CPU is fast enough to violate that without a small delay.
    let nss_delay = Delay::new();
    let itf = SpiInterface::new(spi_dev).with_delay(move || nss_delay.delay_nanos(200));

    let mut rfid = Mfrc522::new(itf).init().expect("RC522 init failed (wiring?)");
    match rfid.version() {
        Ok(v) => info!("RC522 VersionReg = 0x{:02X} (0x91 / 0x92 are genuine)", v),
        Err(_) => warn!("could not read VersionReg"),
    }

    info!("Present a MIFARE card or tag to the reader ...");
    loop {
        // REQA: ask any card in the field to answer; Ok(..) means one did.
        if let Ok(atqa) = rfid.new_card_present() {
            // Anticollision + SELECT -> unique ID (4, 7 or 10 bytes).
            match rfid.select(&atqa) {
                Ok(uid) => {
                    info!("card UID = {:02X?}", uid.as_bytes());
                    let _ = rfid.hlta(); // put the card to sleep so it is read once
                }
                Err(_) => warn!("card detected but SELECT failed"),
            }
        }
        delay.delay_millis(200);
    }
}
