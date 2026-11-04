//! Project 3 - Advanced IoT monitoring system: sensors + LCD + Wi-Fi + Embassy tasks
//!
//! Build:  SSID="MyWifi" PASSWORD="secret" cargo run --release --bin proj3_iot_monitor
//! Then browse to  http://<ip printed on the LCD / serial>/   (auto-refreshing dashboard)
//!                 http://<ip>/api                             (JSON for scripts / Home Assistant)
//!
//! Sensors (all chosen so that they need no I2C address and no Wi-Fi-conflicting ADC2 pin):
//!   LDR light sensor  3V3 --[LDR]--+--[10 k]-- GND,  node -> GPIO1 (ADC1_CH0)
//!   NTC 10 k beta 3950   3V3 --[10 k]--+--[NTC]-- GND,  node -> GPIO2 (ADC1_CH1)
//!   (The on-die temperature sensor is NOT used: esp-hal 1.1.1 has no `tsens` driver for ESP32-S3.)
//! Other pins:
//!   LCD1602 I2C SDA=GPIO41 SCL=GPIO40 (0x27)     alarm LED GPIO21 --[330 ohm]--> GND
//!   page button GPIO14 --> GND (internal pull-up)
//! Camera pins are reserved (moved into `_camera`) and never touched.
//!
//! Task architecture (one cooperative Embassy executor, see ASCII diagram in the manual):
//!   main          sensor loop, 1 Hz: ADC x2 -> LATEST (shared state)
//!   display_task  LCD refresh, 2 Hz, two pages (sensors / network)
//!   button_task   awaits button edge -> toggles PAGE
//!   connection    keeps Wi-Fi associated        net_task  runs the TCP/IP stack
//!   ip_task       waits for DHCP, stores the IP  http_task serves "/" (HTML) and "/api" (JSON)
//! Shared data: `LATEST` (async Mutex), `IPADDR` (async Mutex), `PAGE` (AtomicU8).
//! Not hardware-tested; the NTC/LDR formulas are standard but need calibration for real use.
#![no_std]
#![no_main]

use core::{
    fmt::Write as _,
    sync::atomic::{AtomicU8, Ordering},
};

use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources, tcp::TcpSocket};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Ticker, Timer};
use embedded_io_async::Write as _;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcCalCurve, AdcConfig, Attenuation},
    clock::CpuClock,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::master::{Config as I2cConfig, I2c},
    interrupt::software::SoftwareInterruptControl,
    peripherals::ADC1,
    ram,
    rng::Rng,
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::{Config, ControllerConfig, Interface, WifiController, sta::StationConfig};
use esp32s3_labs::{lcd1602::Lcd1602, reserve_camera_pins};
use log::{info, warn};

esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write($val);
        x
    }};
}

const SSID: &str = match option_env!("SSID") {
    Some(s) => s,
    None => "YOUR_SSID",
};
const PASSWORD: &str = match option_env!("PASSWORD") {
    Some(s) => s,
    None => "YOUR_PASSWORD",
};
const LCD_ADDR: u8 = 0x27;
const ALARM_C: f32 = 30.0; // alarm LED lights above this NTC temperature
const FULL_SCALE_MV: u32 = 3100;

// ------------------------------------------------------------------------------------------
// shared state
// ------------------------------------------------------------------------------------------
#[derive(Clone, Copy)]
struct Reading {
    ntc_c: f32,
    ntc_min: f32,
    ntc_max: f32,
    light_pct: u8,
    light_mv: u16,
    uptime_s: u32,
}

impl Reading {
    const ZERO: Reading = Reading { ntc_c: 0.0, ntc_min: 0.0, ntc_max: 0.0, light_pct: 0, light_mv: 0, uptime_s: 0 };
}

static LATEST: Mutex<CriticalSectionRawMutex, Reading> = Mutex::new(Reading::ZERO);
static IPADDR: Mutex<CriticalSectionRawMutex, Option<[u8; 4]>> = Mutex::new(None);
static PAGE: AtomicU8 = AtomicU8::new(0);

/// Convert the NTC divider voltage (mV, 3V3 -- 10k -- node -- NTC -- GND) to degrees Celsius
/// with the beta equation  1/T = 1/T0 + ln(R/R0)/B.
fn ntc_celsius(mv: u32) -> f32 {
    const VCC_MV: f32 = 3300.0;
    const R_FIXED: f32 = 10_000.0;
    const R0: f32 = 10_000.0; // NTC resistance at T0
    const BETA: f32 = 3950.0;
    const T0_K: f32 = 298.15; // 25 degC
    let v = mv as f32;
    if v < 1.0 || v > VCC_MV - 1.0 {
        return f32::NAN; // open or shorted sensor
    }
    let r_ntc = R_FIXED * v / (VCC_MV - v);
    let inv_t = 1.0 / T0_K + libm::logf(r_ntc / R0) / BETA;
    1.0 / inv_t - 273.15
}

// ------------------------------------------------------------------------------------------
// main = hardware bring-up + sensor loop
// ------------------------------------------------------------------------------------------
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let _camera = reserve_camera_pins!(peripherals);

    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // --- peripherals owned by tasks --------------------------------------------------------
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default().with_frequency(Rate::from_khz(100)))
        .unwrap()
        .with_sda(peripherals.GPIO41)
        .with_scl(peripherals.GPIO40);
    let lcd = Lcd1602::new(i2c, Delay::new(), LCD_ADDR);
    let button = Input::new(peripherals.GPIO14, InputConfig::default().with_pull(Pull::Up));
    let mut alarm = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    // --- peripherals owned by main (the sensor loop) -----------------------------------------
    let mut adc_cfg = AdcConfig::new();
    let mut ldr = adc_cfg.enable_pin_with_cal::<_, AdcCalCurve<ADC1>>(peripherals.GPIO1, Attenuation::_11dB);
    let mut ntc = adc_cfg.enable_pin_with_cal::<_, AdcCalCurve<ADC1>>(peripherals.GPIO2, Attenuation::_11dB);
    let mut adc = Adc::new(peripherals.ADC1, adc_cfg);

    // --- Wi-Fi station + network stack ----------------------------------------------------
    let station = Config::Station(StationConfig::default().with_ssid(SSID).with_password(PASSWORD.into()));
    let (controller, interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, ControllerConfig::default().with_initial_config(station)).unwrap();
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    let (stack, runner) = embassy_net::new(
        interfaces.station,
        embassy_net::Config::dhcpv4(Default::default()),
        mk_static!(StackResources<4>, StackResources::<4>::new()),
        seed,
    );

    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(ip_task(stack).unwrap());
    spawner.spawn(http_task(stack).unwrap());
    spawner.spawn(display_task(lcd).unwrap());
    spawner.spawn(button_task(button).unwrap());

    // --- sensor loop ---------------------------------------------------------------------------
    let boot = Instant::now();
    let (mut t_min, mut t_max) = (f32::MAX, f32::MIN);
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        let (mut l, mut n) = (0u32, 0u32);
        for _ in 0..8 {
            l += nb::block!(adc.read_oneshot(&mut ldr)).unwrap() as u32;
            n += nb::block!(adc.read_oneshot(&mut ntc)).unwrap() as u32;
        }
        let (light_mv, ntc_mv) = (l / 8, n / 8);
        let t = ntc_celsius(ntc_mv);
        if t.is_finite() {
            t_min = t_min.min(t);
            t_max = t_max.max(t);
        }
        let reading = Reading {
            ntc_c: t,
            ntc_min: if t_min <= t_max { t_min } else { 0.0 },
            ntc_max: if t_min <= t_max { t_max } else { 0.0 },
            light_pct: (light_mv * 100 / FULL_SCALE_MV).min(100) as u8,
            light_mv: light_mv as u16,
            uptime_s: boot.elapsed().as_secs() as u32,
        };
        *LATEST.lock().await = reading;
        alarm.set_level(if reading.ntc_c > ALARM_C { Level::High } else { Level::Low });
        info!(
            "ntc {:.1} C (min {:.1}, max {:.1}), light {} % ({} mV)",
            reading.ntc_c, reading.ntc_min, reading.ntc_max, reading.light_pct, reading.light_mv
        );
        ticker.next().await;
    }
}

// ------------------------------------------------------------------------------------------
// tasks
// ------------------------------------------------------------------------------------------
fn pad16(s: &mut heapless::String<24>) {
    while s.len() < 16 {
        let _ = s.push(' ');
    }
    s.truncate(16);
}

#[embassy_executor::task]
async fn display_task(mut lcd: Lcd1602<I2c<'static, Blocking>, Delay>) {
    if lcd.init().is_err() {
        warn!("LCD not found at 0x{:02X}: display task ends (the rest keeps running)", LCD_ADDR);
        return;
    }
    let mut ticker = Ticker::every(Duration::from_millis(500));
    loop {
        let r = *LATEST.lock().await;
        let ip = *IPADDR.lock().await;
        let mut top = heapless::String::<24>::new();
        let mut bottom = heapless::String::<24>::new();
        if PAGE.load(Ordering::Relaxed) % 2 == 0 {
            let _ = write!(top, "T{:5.1}C L{:3}%", r.ntc_c, r.light_pct);
            let _ = write!(bottom, "{:3.0}..{:3.0}C {:5}s", r.ntc_min, r.ntc_max, r.uptime_s); // 16 columns
        } else {
            match ip {
                Some([a, b, c, d]) => {
                    let _ = write!(top, "{}.{}.{}.{}", a, b, c, d);
                    let _ = write!(bottom, "Wi-Fi OK  :80");
                }
                None => {
                    let _ = write!(top, "No IP yet");
                    let _ = write!(bottom, "Wi-Fi connecting");
                }
            }
        }
        pad16(&mut top);
        pad16(&mut bottom);
        let _ = lcd.set_cursor(0, 0);
        let _ = lcd.print(&top);
        let _ = lcd.set_cursor(0, 1);
        let _ = lcd.print(&bottom);
        ticker.next().await;
    }
}

#[embassy_executor::task]
async fn button_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        PAGE.fetch_xor(1, Ordering::Relaxed);
        Timer::after_millis(250).await; // debounce
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        match controller.connect_async().await {
            Ok(info) => {
                info!("Wi-Fi connected: {:?}", info);
                let reason = controller.wait_for_disconnect_async().await.ok();
                warn!("Wi-Fi disconnected: {:?}", reason);
                *IPADDR.lock().await = None;
            }
            Err(e) => warn!("Wi-Fi connect failed: {:?}", e),
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn ip_task(stack: Stack<'static>) {
    loop {
        stack.wait_config_up().await;
        if let Some(cfg) = stack.config_v4() {
            let o = cfg.address.address().octets();
            info!("Got IP {}.{}.{}.{}", o[0], o[1], o[2], o[3]);
            *IPADDR.lock().await = Some(o);
        }
        stack.wait_config_down().await; // loop again after the next (re)connect
    }
}

#[embassy_executor::task]
async fn http_task(stack: Stack<'static>) {
    let mut rx = [0u8; 512];
    let mut tx = [0u8; 1024];
    loop {
        let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
        socket.set_timeout(Some(Duration::from_secs(10)));
        if socket.accept(80).await.is_err() {
            continue;
        }

        let mut req = [0u8; 256];
        let n = socket.read(&mut req).await.unwrap_or(0);
        let r = *LATEST.lock().await;
        let ntc = if r.ntc_c.is_finite() { r.ntc_c } else { -999.0 };

        let wants_json = req[..n].starts_with(b"GET /api");
        let mut body = heapless::String::<400>::new();
        if wants_json {
            let _ = write!(
                body,
                "{{\"ntc_c\":{:.1},\"ntc_min\":{:.1},\"ntc_max\":{:.1},\"light_pct\":{},\"light_mv\":{},\"uptime_s\":{}}}",
                ntc, r.ntc_min, r.ntc_max, r.light_pct, r.light_mv, r.uptime_s
            );
        } else {
            let _ = write!(
                body,
                "<!doctype html><meta charset=utf-8><meta http-equiv=refresh content=2>\
                 <title>ESP32-S3 monitor</title><h1>ESP32-S3 monitor</h1>\
                 <p>Temperature (NTC): {:.1} &deg;C<p>Session min / max: {:.1} / {:.1} &deg;C\
                 <p>Light: {} % ({} mV)<p>Uptime: {} s",
                ntc, r.ntc_min, r.ntc_max, r.light_pct, r.light_mv, r.uptime_s
            );
        }
        let mut resp = heapless::String::<640>::new();
        let _ = write!(
            resp,
            "HTTP/1.0 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            if wants_json { "application/json" } else { "text/html" },
            body.len(),
            body
        );
        let _ = socket.write_all(resp.as_bytes()).await;
        let _ = socket.flush().await;
        socket.close();
        Timer::after_millis(50).await; // let the stack send the FIN before reusing buffers
    }
}
