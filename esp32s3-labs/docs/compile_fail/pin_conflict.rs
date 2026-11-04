#![no_std]
#![no_main]
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp32s3_labs::reserve_camera_pins;
esp_bootloader_esp_idf::esp_app_desc!();
#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _camera = reserve_camera_pins!(peripherals);
    // GPIO15 is the camera XCLK pin, already moved into `_camera`:
    let _bad = Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default());
    loop {}
}
