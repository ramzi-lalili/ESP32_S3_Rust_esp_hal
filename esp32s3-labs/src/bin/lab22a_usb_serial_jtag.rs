//! Lab 22a - USB Serial/JTAG: blocking echo (upper-case converter)
//!
//! Wiring: none - use the board's native USB connector (GPIO19 = D-, GPIO20 = D+).
//! Open the CDC serial port with `espflash monitor`, `picocom` or any terminal and type.
//! This lab deliberately does NOT initialise the esp-println logger: on a board whose USB
//! port is wired to the USB Serial/JTAG controller, logging would share this very channel.
//! API level: UNSTABLE (`esp_hal::usb_serial_jtag`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{main, usb_serial_jtag::UsbSerialJtag};
use nb::block;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut usb = UsbSerialJtag::new(peripherals.USB_DEVICE);

    usb.write(b"\r\nLab 22a: type something, it comes back in UPPER CASE\r\n").ok();
    loop {
        // read_byte() is non-blocking (nb): block! spins until a byte arrives.
        let byte = block!(usb.read_byte()).unwrap();
        let out = match byte {
            b'\r' => b'\n', // terminal sends CR on Enter
            c => c.to_ascii_uppercase(),
        };
        if out == b'\n' {
            usb.write(b"\r\n").ok();
        } else {
            usb.write(&[out]).ok();
        }
    }
}
