//! Project 2 - RFID access-control system: RC522 + LCD1602 + servo door lock
//!
//! A card is read by the RC522; if its UID is on the whitelist the servo "opens the door"
//! for 3 s and a green LED lights, otherwise "ACCESS DENIED" is shown.
//!
//! Pin allocation (all distinct; camera pins reserved and untouched; LCD bus = GPIO40/41):
//!   RC522  SDA(CS)=GPIO21  SCK=GPIO39  MOSI=GPIO38  MISO=GPIO42  RST=GPIO14   3.3 V supply!
//!   LCD    SDA=GPIO41  SCL=GPIO40  (I2C0, address 0x27)
//!   Servo  signal=GPIO47 (LEDC 50 Hz)  +5 V from an external supply, common GND
//!   LED    GPIO1 --[330 ohm]--> LED --> GND  (door-open indicator)
//!
//! Software architecture: a small finite-state machine polled every 100 ms
//!   Idle --(card seen)--> Granted | Denied --(3 s / 2 s elapsed)--> Idle
//! The reader is only polled in `Idle`, which also debounces repeated reads of the same card.
//! Authorisation is a whitelist of UIDs below. Run once, read the UID from the serial log of a
//! denied card, paste it into WHITELIST, rebuild.
//! SECURITY NOTE: MIFARE UIDs can be cloned; this is a teaching project, not a secure lock.
//! API level: SPI/I2C STABLE; LEDC/Delay UNSTABLE; mfrc522 0.8.0 (not hardware-tested).
#![no_std]
#![no_main]


use embedded_hal::pwm::SetDutyCycle;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{DriveMode, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    ledc::{
        LSGlobalClkSource, Ledc, LowSpeed,
        channel::{self, ChannelIFace},
        timer::{self, TimerIFace},
    },
    main,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::{Duration, Instant, Rate},
};
use esp32s3_labs::{lcd1602::Lcd1602, reserve_camera_pins};
use log::{info, warn};
use mfrc522::{Mfrc522, comm::blocking::spi::SpiInterface};

esp_bootloader_esp_idf::esp_app_desc!();

/// Authorised card UIDs (4, 7 or 10 bytes each). Replace with your own cards.
const WHITELIST: &[&[u8]] = &[
    &[0xDE, 0xAD, 0xBE, 0xEF],
    &[0x01, 0x23, 0x45, 0x67],
];

const LCD_ADDR: u8 = 0x27;
const OPEN_FOR: Duration = Duration::from_millis(3000);
const DENIED_FOR: Duration = Duration::from_millis(2000);

enum State {
    Idle,
    Granted(Instant),
    Denied(Instant),
}

/// 0..=180 degrees -> 0.5..2.5 ms pulse in a 20 ms period.
fn set_angle<S: SetDutyCycle>(servo: &mut S, deg: u32) {
    let pulse_us = 500 + 2000 * deg.min(180) / 180;
    let ticks = pulse_us * servo.max_duty_cycle() as u32 / 20_000;
    let _ = servo.set_duty_cycle(ticks as u16);
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _camera = reserve_camera_pins!(peripherals);
    let delay = Delay::new();

    // --- outputs: LED + servo ----------------------------------------------------------------
    let mut led = Output::new(peripherals.GPIO1, Level::Low, OutputConfig::default());

    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut timer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    timer0
        .configure(timer::config::Config {
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
    set_angle(&mut servo, 0); // locked

    // --- LCD ------------------------------------------------------------------------------------
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default().with_frequency(Rate::from_khz(100)))
        .unwrap()
        .with_sda(peripherals.GPIO41)
        .with_scl(peripherals.GPIO40);
    let mut lcd = Lcd1602::new(i2c, delay, LCD_ADDR);
    let lcd_ok = lcd.init().is_ok();
    if !lcd_ok {
        warn!("LCD not found: serial output only");
    }
    let mut show = |top: &str, bottom: &str| {
        if lcd_ok {
            let _ = lcd.clear();
            let _ = lcd.print(top);
            let _ = lcd.set_cursor(0, 1);
            let _ = lcd.print(bottom);
        }
    };

    // --- RC522 ----------------------------------------------------------------------------------
    let mut rst = Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default());
    delay.delay_millis(50);
    rst.set_high();
    delay.delay_millis(50);

    let spi_bus = Spi::new(
        peripherals.SPI2,
        SpiConfig::default().with_frequency(Rate::from_mhz(4)).with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO39)
    .with_mosi(peripherals.GPIO38)
    .with_miso(peripherals.GPIO42);
    let cs = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());
    let spi_dev = ExclusiveDevice::new(spi_bus, cs, Delay::new()).unwrap();
    let nss_delay = Delay::new();
    let itf = SpiInterface::new(spi_dev).with_delay(move || nss_delay.delay_nanos(200));
    let mut rfid = Mfrc522::new(itf).init().expect("RC522 init failed - check SPI wiring and 3.3 V");
    info!("RC522 version register: {:?}", rfid.version().ok());

    show("Access control", "Present card...");
    let mut state = State::Idle;

    loop {
        match state {
            State::Idle => {
                if let Ok(atqa) = rfid.new_card_present() {
                    if let Ok(uid) = rfid.select(&atqa) {
                        let bytes = uid.as_bytes();
                        let _ = rfid.hlta(); // card sleeps until it leaves and re-enters the field
                        if WHITELIST.iter().any(|allowed| *allowed == bytes) {
                            info!("ACCESS GRANTED, UID {:02X?}", bytes);
                            show("Access GRANTED", "Welcome!");
                            set_angle(&mut servo, 90); // unlock
                            led.set_high();
                            state = State::Granted(Instant::now());
                        } else {
                            warn!("ACCESS DENIED, UID {:02X?}", bytes);
                            show("Access DENIED", "Unknown card");
                            state = State::Denied(Instant::now());
                        }
                    }
                }
            }
            State::Granted(since) => {
                if since.elapsed() >= OPEN_FOR {
                    set_angle(&mut servo, 0); // lock again
                    led.set_low();
                    show("Access control", "Present card...");
                    state = State::Idle;
                }
            }
            State::Denied(since) => {
                if since.elapsed() >= DENIED_FOR {
                    show("Access control", "Present card...");
                    state = State::Idle;
                }
            }
        }
        delay.delay_millis(100);
    }
}
