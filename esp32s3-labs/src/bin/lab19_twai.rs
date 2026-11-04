//! Lab 19 - TWAI (CAN 2.0) communication
//!
//! TWAI is Espressif's name for the CAN 2.0 controller. This lab runs in SELF-TEST mode:
//! the controller sends a frame and receives its own copy, so it needs NO second node and
//! NO transceiver. Wiring: nothing but GPIO21 (it acts as both TX and RX, open-drain with
//! pull-up, via `new_no_transceiver`).
//!
//! For a real bus use `TwaiConfiguration::new(TWAI0, rx, tx, ...)` with a 3.3 V transceiver
//! (e.g. SN65HVD230): GPIO21 -> TXD, GPIO14 <- RXD, 120 ohm termination at both bus ends,
//! and switch `TwaiMode::SelfTest` to `TwaiMode::Normal`.
//! API level: UNSTABLE (`esp_hal::twai`). Needs `nb` for the non-blocking API.
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    main,
    twai::{self, EspTwaiFrame, StandardId, TwaiMode, filter::SingleStandardFilter},
};
use log::info;
use nb::block;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // One physical pin used as both RX and TX (no transceiver).
    // `split` is unsafe because it hands out two handles to one GPIO.
    let (rx_pin, tx_pin) = unsafe { peripherals.GPIO21.split() };

    let mut config = twai::TwaiConfiguration::new_no_transceiver(
        peripherals.TWAI0,
        rx_pin,
        tx_pin,
        twai::BaudRate::B125K,
        TwaiMode::SelfTest,
    );

    // Accept every standard (11-bit) identifier: 'x' = don't care.
    config.set_filter(
        const { SingleStandardFilter::new(b"xxxxxxxxxxx", b"x", [b"xxxxxxxx", b"xxxxxxxx"]) },
    );

    // start() leaves configuration mode: baud rate and filter are now locked.
    let mut can = config.start();

    let mut n: u8 = 0;
    loop {
        // In self-test mode the frame must be created with `new_self_reception`
        // so that the controller also loops it back into its RX FIFO.
        let tx = EspTwaiFrame::new_self_reception(StandardId::new(0x123).unwrap(), &[n, 0xDE, 0xAD]).unwrap();
        block!(can.transmit(&tx)).unwrap();

        let rx = block!(can.receive()).unwrap();
        info!("sent #{} and received back: {:?}", n, rx);
        info!(
            "error counters  TX={}  RX={}",
            can.transmit_error_count(),
            can.receive_error_count()
        );

        n = n.wrapping_add(1);
        delay.delay_millis(1000);
    }
}
