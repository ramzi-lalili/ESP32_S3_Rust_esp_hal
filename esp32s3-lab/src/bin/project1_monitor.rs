#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

use core::fmt::Write;
use esp32s3_lab::lcd1602::Lcd;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    delay::Delay,
    gpio::DriveMode,
    i2c::master::{Config as I2cConfig, I2c},
    ledc::{
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
        LSGlobalClkSource, Ledc, LowSpeed,
    },
    main,
    time::Rate,
};

// Pins: ADC sensor GPIO1 | LED (PWM) GPIO2 | LCD SDA GPIO41, SCL GPIO40
#[main]
fn main() -> ! {
    let p = esp_hal::init(esp_hal::Config::default());

    let mut adc_cfg = AdcConfig::new();
    let mut sensor = adc_cfg.enable_pin(p.GPIO1, Attenuation::_11dB);
    let mut adc1 = Adc::new(p.ADC1, adc_cfg);

    let mut ledc = Ledc::new(p.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut t0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    t0.configure(timer::config::Config {
        duty: timer::config::Duty::Duty8Bit,
        clock_source: timer::LSClockSource::APBClk,
        frequency: Rate::from_khz(5),
    })
    .unwrap();
    let mut led = ledc.channel::<LowSpeed>(channel::Number::Channel0, p.GPIO2);
    led.configure(channel::config::Config {
        timer: &t0,
        duty_pct: 0,
        drive_mode: DriveMode::PushPull,
    })
    .unwrap();

    let i2c = I2c::new(p.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(p.GPIO41)
        .with_scl(p.GPIO40);
    let mut lcd = Lcd::new(i2c, 0x27);
    lcd.init();

    let delay = Delay::new();
    loop {
        let raw: u16 = nb::block!(adc1.read_oneshot(&mut sensor)).unwrap();
        let pct = ((raw as u32 * 100) / 4095).min(100) as u8;
        led.set_duty(pct).unwrap();

        lcd.set_cursor(0, 0);
        write!(lcd, "ADC:{:4} {:3}% ", raw, pct).ok();
        lcd.set_cursor(0, 1);
        write!(lcd, "LED duty {:3}%  ", pct).ok();
        println!("raw={} pct={}", raw, pct);
        delay.delay_millis(200);
    }
}
