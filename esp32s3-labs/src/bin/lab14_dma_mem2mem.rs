//! Lab 14a - DMA memory-to-memory transfer
//!
//! No wiring needed. The DMA controller copies a 10 KiB buffer without CPU help.
//! NOTE (ESP32-S3): `Mem2Mem::new` consumes DMA_CH0 *and* SPI2, because the
//! copy path is routed through the SPI2 DMA peripheral. SPI2 is therefore not
//! available to the same program afterwards.
//! API level: UNSTABLE (`esp_hal::dma`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    dma::{BurstConfig, Mem2Mem},
    dma_buffers, main,
    time::{Duration, Instant},
};
use log::{error, info};

esp_bootloader_esp_idf::esp_app_desc!();

const DATA_SIZE: usize = 10 * 1024;

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // The macro creates static buffers + DMA descriptors (linked list entries,
    // each describing <= 4092 bytes) and returns `&'static mut` references.
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(DATA_SIZE);

    let mem2mem = Mem2Mem::new(peripherals.DMA_CH0, peripherals.SPI2);
    let mut mem2mem = mem2mem
        .with_descriptors(rx_descriptors, tx_descriptors, BurstConfig::default())
        .unwrap();

    for (i, b) in tx_buffer.iter_mut().enumerate() {
        *b = (i % 251) as u8; // 251 is prime -> pattern does not repeat at 256
    }

    info!("starting DMA copy of {} bytes", DATA_SIZE);
    let t0 = Instant::now();
    match mem2mem.start_transfer(rx_buffer, tx_buffer) {
        Ok(transfer) => {
            transfer.wait().unwrap(); // blocks until the DMA "done" interrupt flag
            let us = t0.elapsed().as_micros();
            let ok = rx_buffer.iter().zip(tx_buffer.iter()).all(|(a, b)| a == b);
            if ok {
                info!("DMA copy verified OK in {} us", us);
            } else {
                error!("DMA copy MISMATCH");
            }
        }
        Err(e) => error!("start_transfer failed: {:?}", e),
    }

    loop {
        delay.delay(Duration::from_secs(2));
    }
}
