//! Lab 16 - RMT peripheral: driving a WS2812 ("NeoPixel") RGB LED
//!
//! Wiring: WS2812 DIN -> GPIO48 (on many ESP32-S3 boards an RGB LED is already wired to
//! GPIO48 - check your schematic; some use GPIO38). WS2812 VDD 5 V (3.3 V often works for a
//! single LED), GND common. The LED's data input expects ~0.7*VDD high level, so with a
//! 5 V LED use a level shifter (e.g. 74AHCT125) for reliable operation.
//!
//! The RMT peripheral turns a list of `PulseCode`s (level/duration pairs) into an
//! exact waveform without CPU involvement - ideal for the 800 kHz one-wire WS2812
//! protocol (each bit = one high pulse + one low pulse of different lengths).
//! API level: UNSTABLE (`esp_hal::rmt`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::Level,
    main,
    rmt::{PulseCode, Rmt, TxChannelConfig, TxChannelCreator},
    time::Rate,
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

// RMT tick = 1 / 80 MHz = 12.5 ns (clk_divider = 1).
const T0H: u16 = 32; // 0.40 us
const T0L: u16 = 68; // 0.85 us
const T1H: u16 = 64; // 0.80 us
const T1L: u16 = 36; // 0.45 us

/// Convert one RGB colour to 24 pulse codes (WS2812 expects the order G, R, B,
/// most significant bit first) plus the end marker that terminates the frame.
fn encode(r: u8, g: u8, b: u8) -> [PulseCode; 25] {
    let bit0 = PulseCode::new(Level::High, T0H, Level::Low, T0L);
    let bit1 = PulseCode::new(Level::High, T1H, Level::Low, T1L);
    let mut out = [PulseCode::end_marker(); 25];
    let word = ((g as u32) << 16) | ((r as u32) << 8) | b as u32;
    for i in 0..24 {
        out[i] = if word & (1 << (23 - i)) != 0 { bit1 } else { bit0 };
    }
    out
}

/// Integer HSV (h in 0..=255 around the colour wheel, full saturation) -> RGB.
fn wheel(h: u8, brightness: u8) -> (u8, u8, u8) {
    let region = h / 43;
    let rem = (h as u16 - region as u16 * 43) * 6; // 0..=252
    let (r, g, b) = match region {
        0 => (255, rem as u8, 0),
        1 => (255 - rem as u8, 255, 0),
        2 => (0, 255, rem as u8),
        3 => (0, 255 - rem as u8, 255),
        4 => (rem as u8, 0, 255),
        _ => (255, 0, 255 - rem as u8),
    };
    let s = |c: u8| ((c as u16 * brightness as u16) / 255) as u8;
    (s(r), s(g), s(b))
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // RMT source clock on ESP32-S3 is 80 MHz.
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let mut channel = rmt
        .channel0
        .configure_tx(&TxChannelConfig::default().with_clk_divider(1))
        .unwrap()
        .with_pin(peripherals.GPIO48);

    info!("Lab 16: rainbow on the WS2812 (GPIO48)");
    let mut hue: u8 = 0;
    loop {
        let (r, g, b) = wheel(hue, 32); // keep it dim: 32/255
        let data = encode(r, g, b);

        // transmit() consumes the channel and returns a transaction; wait()
        // returns the channel when the frame (and its end marker) has been sent.
        let transaction = channel.transmit(&data).unwrap();
        channel = transaction.wait().unwrap();

        hue = hue.wrapping_add(2);
        delay.delay_millis(20); // also provides the >50 us reset/latch gap
    }
}
