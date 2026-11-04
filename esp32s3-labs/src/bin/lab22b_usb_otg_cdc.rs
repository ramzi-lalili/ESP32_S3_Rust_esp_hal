//! Lab 22b - USB OTG (full-speed device): CDC-ACM virtual serial port with embassy-usb
//!
//! Adapted from the official esp-hal-v1.1.1 example `examples/async/embassy_usb_serial`.
//! The USB OTG FS core uses the SAME two pins as USB Serial/JTAG: GPIO19 (D-) and GPIO20 (D+).
//! Consequences you must understand:
//!  * While this firmware runs, the USB Serial/JTAG logger/flash path on that connector is gone.
//!    Console output (esp-println) goes to UART0 only if you build with the `uart` feature of
//!    esp-println and use a USB-UART bridge on GPIO43/44. We use plain `println!` here, which
//!    in the default `auto` mode is routed to the USB Serial/JTAG block and may therefore be lost.
//!  * To re-flash, hold BOOT, tap RESET (enter ROM download mode), then flash again.
//! The enumerated device appears as "USB-serial example" (VID 0x303A, PID 0x3001); anything
//! you type is echoed back in upper case.
//! API level: UNSTABLE (`esp_hal::otg_fs`) + embassy-usb 0.6.
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_usb::{
    Builder,
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
};
use esp_backtrace as _;
use esp_hal::{
    interrupt::software::SoftwareInterruptControl,
    otg_fs::{
        Usb,
        asynch::{Config, Driver},
    },
    timer::timg::TimerGroup,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_alloc::heap_allocator!(size: 16 * 1024); // see Cargo.toml note: esp-rtos alloc support is on

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // Takes the USB0 peripheral plus its two data pins (note the order: D+ first).
    let usb = Usb::new(peripherals.USB0, peripherals.GPIO20, peripherals.GPIO19);

    let mut ep_out_buffer = [0u8; 1024];
    let driver = Driver::new(usb, &mut ep_out_buffer, Config::default());

    let mut config = embassy_usb::Config::new(0x303A, 0x3001);
    config.manufacturer = Some("Espressif");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    // Required for Windows compatibility (interface association descriptors).
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no Microsoft OS descriptors
        &mut control_buf,
    );
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);
    let mut usb = builder.build();

    let usb_fut = usb.run(); // enumeration, control transfers, ...
    let echo_fut = async {
        loop {
            class.wait_connection().await;
            let _ = echo(&mut class).await;
        }
    };
    join(usb_fut, echo_fut).await;
}

async fn echo<'d>(class: &mut CdcAcmClass<'d, Driver<'d>>) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        let n = class.read_packet(&mut buf).await?;
        for c in buf[..n].iter_mut() {
            if c.is_ascii_lowercase() {
                *c = c.to_ascii_uppercase();
            }
        }
        class.write_packet(&buf[..n]).await?;
    }
}

struct Disconnected;

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("USB buffer overflow"),
            EndpointError::Disabled => Disconnected,
        }
    }
}
