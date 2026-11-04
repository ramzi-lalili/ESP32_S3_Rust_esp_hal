//! Lab 09 - UART serial communication (loopback self-test)
//!
//! Wiring: UART1  TX = GPIO47, RX = GPIO42. Connect GPIO47 to GPIO42 with ONE
//! jumper wire: every byte sent is received again.
//! (UART0 on GPIO43/GPIO44 is left alone: it usually carries the console.)
//! To talk to a PC instead, use a 3.3 V USB-UART adapter: TX->RX, RX->TX, GND->GND.
//! API level: STABLE (`esp_hal::uart`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    main,
    time::{Duration, Instant},
    uart::{Config, Uart},
};
use log::{error, info};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // 115200 baud, 8 data bits, no parity, 1 stop bit are the defaults except
    // for the baud rate, which we set explicitly for clarity.
    let mut uart = Uart::new(peripherals.UART1, Config::default().with_baudrate(115_200))
        .unwrap()
        .with_tx(peripherals.GPIO47)
        .with_rx(peripherals.GPIO42);

    let mut counter: u32 = 0;
    loop {
        // --- transmit -------------------------------------------------------
        let mut msg = heapless::String::<32>::new();
        core::fmt::write(&mut msg, format_args!("Hello UART #{}\r\n", counter)).unwrap();
        let sent = uart.write(msg.as_bytes()).unwrap(); // bytes accepted by the FIFO
        uart.flush().unwrap(); // wait until the last bit left the wire
        counter += 1;

        // --- receive (poll with a 200 ms timeout) ----------------------------
        let start = Instant::now();
        let mut buf = [0u8; 32];
        let mut got = 0;
        while got < sent && start.elapsed() < Duration::from_millis(200) {
            if uart.read_ready() {
                got += uart.read(&mut buf[got..]).unwrap();
            }
        }

        if got == sent && buf[..got] == *msg.as_bytes() {
            info!("loopback OK ({} bytes): {:?}", got, core::str::from_utf8(&buf[..got]).unwrap_or("?").trim_end());
        } else {
            error!("loopback FAILED: sent {} bytes, received {} - is the jumper fitted?", sent, got);
        }

        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(1) {}
    }
}
