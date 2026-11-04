//! Lab 14b - SPI + DMA full-duplex loopback
//!
//! Wiring: SCK=GPIO39, MOSI=GPIO38, MISO=GPIO42, CS=GPIO21, jumper GPIO38 <-> GPIO42.
//! With DMA the CPU only prepares buffers; the transfer runs in the background
//! and `wait()` hands the buffers (and the SPI driver) back when finished.
//! API level: UNSTABLE (`esp_hal::dma`, `Spi::with_dma`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers, main,
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};
use log::{error, info};

esp_bootloader_esp_idf::esp_app_desc!();

const BUF: usize = 256;

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(BUF);
    let mut dma_rx = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let mut dma_tx = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let mut spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO39)
    .with_mosi(peripherals.GPIO38)
    .with_miso(peripherals.GPIO42)
    .with_cs(peripherals.GPIO21)
    .with_dma(peripherals.DMA_CH0); // SpiDma: DMA-capable driver

    let mut round: u8 = 0;
    loop {
        for (i, b) in dma_tx.as_mut_slice().iter_mut().enumerate() {
            *b = (i as u8).wrapping_add(round);
        }

        // Ownership: the driver and both buffers are MOVED into the transfer and
        // given back by wait(); the compiler therefore forbids touching them
        // while the DMA engine is still reading/writing them.
        let transfer = spi
            .transfer(dma_rx.len(), dma_rx, dma_tx.len(), dma_tx)
            .map_err(|e| e.0)
            .unwrap();
        (spi, (dma_rx, dma_tx)) = transfer.wait();

        let ok = dma_rx.as_slice() == dma_tx.as_slice();
        if ok {
            info!("SPI-DMA loopback OK, first bytes {:02X?}", &dma_rx.as_slice()[..8]);
        } else {
            error!("SPI-DMA loopback MISMATCH - jumper GPIO38<->GPIO42 fitted?");
        }
        round = round.wrapping_add(1);
        delay.delay_millis(1000);
    }
}
