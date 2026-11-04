#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    delay::Delay,
    main,
    uart::{Config, Uart},
};

// Loopback test: wire GPIO47 (TX) to GPIO48 (RX) with a jumper.
#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut uart = Uart::new(p.UART1, Config::default().with_baudrate(115_200))
        .unwrap()
        .with_tx(p.GPIO47)
        .with_rx(p.GPIO48);
    let delay = Delay::new();

    loop {
        let msg = b"hello";
        let _ = uart.write(msg).unwrap();
        uart.flush().unwrap();

        let mut buf = [0u8; 5];
        let n = uart.read(&mut buf).unwrap(); // blocks until at least one byte arrives
        println!("received {} byte(s): {:?}", n, &buf[..n]);
        delay.delay_millis(1000);
    }
}
