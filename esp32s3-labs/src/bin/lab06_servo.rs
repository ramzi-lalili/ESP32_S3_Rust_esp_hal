//! Lab 06 - Servo motor control with PWM (LEDC, 50 Hz)
//!
//! Wiring (SG90 / MG90S class hobby servo):
//!   servo signal (orange/white) -> GPIO47
//!   servo +5 V   (red)          -> EXTERNAL 5 V supply (NOT the 3V3 pin)
//!   servo GND    (brown/black)  -> supply GND AND ESP32-S3 GND (common ground!)
//!
//! A hobby servo wants one pulse every 20 ms; the pulse width sets the angle:
//!   0.5 ms ~ 0 deg, 1.5 ms ~ 90 deg, 2.5 ms ~ 180 deg (typical; check your datasheet).
//! API level: UNSTABLE (`esp_hal::ledc`). Uses the embedded-hal `SetDutyCycle` trait.
#![no_std]
#![no_main]

use embedded_hal::pwm::SetDutyCycle;
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

const PERIOD_US: u32 = 20_000; // 50 Hz
const MIN_US: u32 = 500; // pulse for 0 degrees
const MAX_US: u32 = 2_500; // pulse for 180 degrees

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut timer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    timer0
        .configure(timer::config::Config {
            // 14 bits = 16384 steps per 20 ms period -> ~1.2 us resolution.
            duty: timer::config::Duty::Duty14Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(50),
        })
        .unwrap();

    let mut servo = ledc.channel(channel::Number::Channel0, peripherals.GPIO47);
    servo
        .configure(channel::config::Config {
            timer: &timer0,
            duty_pct: 0,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();

    // embedded-hal: duty is expressed against max_duty_cycle() (= 2^14 here).
    let max = servo.max_duty_cycle() as u32;
    info!("max duty = {} counts per {} us period", max, PERIOD_US);

    let angle_to_ticks = |deg: u32| -> u16 {
        let pulse_us = MIN_US + (MAX_US - MIN_US) * deg.min(180) / 180;
        (pulse_us * max / PERIOD_US) as u16
    };

    loop {
        for deg in (0..=180).step_by(30) {
            servo.set_duty_cycle(angle_to_ticks(deg)).unwrap();
            info!("angle = {} deg", deg);
            delay.delay_millis(600);
        }
        for deg in (0..=180).rev().step_by(30) {
            servo.set_duty_cycle(angle_to_ticks(deg)).unwrap();
            info!("angle = {} deg", deg);
            delay.delay_millis(600);
        }
    }
}
