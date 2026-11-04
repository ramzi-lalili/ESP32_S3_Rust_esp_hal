//! Project 1 - Basic monitoring system: analog sensor -> PWM LED -> LCD1602
//!
//! A light-dependent resistor (LDR) is read with the ADC. A "night light" LED gets BRIGHTER
//! as the room gets DARKER (PWM duty = 100 % - light %), and the LCD shows the measurements.
//!
//! Pin allocation (no pin is used twice; the camera pins are reserved and untouched):
//!   GPIO1  (ADC1_CH0)  LDR divider output      3V3 --[LDR]--+--[10 k]-- GND, node -> GPIO1
//!   GPIO21 (LEDC ch0)  PWM LED                 GPIO21 --[330 ohm]--> LED --> GND
//!   GPIO41 (I2C0 SDA)  LCD1602 backpack SDA    GPIO40 (I2C0 SCL)  LCD1602 backpack SCL
//!
//! Software architecture: one cooperative super-loop
//!   sample (8x oversampling, factory-calibrated mV)  ->  filter (exponential average)
//!   ->  actuate (LEDC duty with square-law gamma)    ->  display (LCD) + log (serial)
//! API level: ADC/LEDC/Delay UNSTABLE; I2C STABLE. LCD driver: src/lcd1602.rs (not hardware-tested).
#![no_std]
#![no_main]

use core::fmt::Write;

use esp_backtrace as _;
use esp_hal::{
    analog::adc::{Adc, AdcCalCurve, AdcConfig, Attenuation},
    delay::Delay,
    gpio::DriveMode,
    i2c::master::{Config as I2cConfig, I2c},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    main,
    peripherals::ADC1,
    time::Rate,
};
use esp32s3_labs::{lcd1602::Lcd1602, reserve_camera_pins};
use log::{info, warn};

esp_bootloader_esp_idf::esp_app_desc!();

const LCD_ADDR: u8 = 0x27;
const OVERSAMPLE: u32 = 8;
/// With 11 dB attenuation the calibrated ADC reads up to about 3.1 V.
const FULL_SCALE_MV: u32 = 3100;

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _camera = reserve_camera_pins!(peripherals);
    let delay = Delay::new();

    // --- ADC1 on GPIO1 --------------------------------------------------------------------
    let mut adc_cfg = AdcConfig::new();
    let mut ldr = adc_cfg
        .enable_pin_with_cal::<_, AdcCalCurve<ADC1>>(peripherals.GPIO1, Attenuation::_11dB);
    let mut adc = Adc::new(peripherals.ADC1, adc_cfg);

    // --- LEDC PWM on GPIO21 (5 kHz, 10-bit) -----------------------------------------------
    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut timer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    timer0
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty10Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_khz(5),
        })
        .unwrap();
    let mut led = ledc.channel(channel::Number::Channel0, peripherals.GPIO21);
    led.configure(channel::config::Config {
        timer: &timer0,
        duty_pct: 0,
        drive_mode: DriveMode::PushPull,
    })
    .unwrap();

    // --- LCD1602 on I2C0 (GPIO41 = SDA, GPIO40 = SCL) -------------------------------------
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default().with_frequency(Rate::from_khz(100)))
        .unwrap()
        .with_sda(peripherals.GPIO41)
        .with_scl(peripherals.GPIO40);
    let mut lcd = Lcd1602::new(i2c, delay, LCD_ADDR);
    let lcd_ok = lcd.init().is_ok();
    if !lcd_ok {
        warn!("LCD not found at 0x{:02X}: continuing with serial output only", LCD_ADDR);
    }

    let mut filtered: i32 = -1; // percent, -1 = no sample yet
    loop {
        // 1) sample ------------------------------------------------------------------------
        let mut sum = 0u32;
        for _ in 0..OVERSAMPLE {
            sum += nb::block!(adc.read_oneshot(&mut ldr)).unwrap() as u32;
        }
        let mv = sum / OVERSAMPLE;
        let light_pct = (mv * 100 / FULL_SCALE_MV).min(100) as i32;

        // 2) filter: first-order IIR, y += (x - y) / 4 -------------------------------------
        filtered = if filtered < 0 { light_pct } else { filtered + (light_pct - filtered) / 4 };

        // 3) actuate: darker room -> brighter LED; square-law gamma for a perceptually
        //    smoother response (see Lab 05).
        let want = (100 - filtered).clamp(0, 100) as u32;
        let duty = (want * want / 100) as u8;
        led.set_duty(duty).unwrap();

        // 4) display + log -------------------------------------------------------------------
        if lcd_ok {
            let _ = lcd.set_cursor(0, 0);
            let _ = write!(lcd, "Light{:3}%{:5}mV", filtered, mv);
            let _ = lcd.set_cursor(0, 1);
            let _ = write!(lcd, "LED duty {:3}%   ", duty);
        }
        info!("ldr = {} mV, light = {} %, led duty = {} %", mv, filtered, duty);

        delay.delay_millis(200);
    }
}
