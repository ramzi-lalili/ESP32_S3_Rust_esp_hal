//! Lab 05 - LEDC PWM: LED brightness control
//!
//! Wiring: GPIO21 --[330 ohm]--> LED --> GND.
//! The LEDC peripheral generates PWM in hardware; the CPU only writes duty values.
//! API level: UNSTABLE (`esp_hal::ledc`).
#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::DriveMode,
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    main,
    time::Rate,
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    // 1) A timer defines frequency and resolution (shared by up to 4 channels).
    let mut timer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    timer0
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty10Bit, // 1024 duty steps
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(5), // 5 kHz: flicker-free for the eye
        })
        .unwrap();

    // 2) A channel binds a timer to an output pin.
    let mut channel0 = ledc.channel(channel::Number::Channel0, peripherals.GPIO21);
    channel0
        .configure(channel::config::Config {
            timer: &timer0,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();

    info!("Lab 05: breathing LED on GPIO21");
    loop {
        // Hardware fade 0 % -> 100 % in 1500 ms, then back.
        channel0.start_duty_fade(0, 100, 1500).unwrap();
        while channel0.is_duty_fade_running() {}
        channel0.start_duty_fade(100, 0, 1500).unwrap();
        while channel0.is_duty_fade_running() {}

        // Manual stepping with a square-law "gamma" curve: perceived brightness
        // is roughly logarithmic, so duty = x^2 looks more linear than duty = x.
        for step in 0..=100u32 {
            channel0.set_duty((step * step / 100) as u8).unwrap();
            delay.delay_millis(15);
        }
        info!("cycle finished");
    }
}
