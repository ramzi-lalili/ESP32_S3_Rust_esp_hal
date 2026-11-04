//! Lab 20 - Wi-Fi connectivity (station mode, DHCP, TCP echo server)
//!
//! Build with your network credentials in environment variables:
//!   SSID="MyWifi" PASSWORD="secret" cargo run --release --bin lab20_wifi
//! (2.4 GHz networks only: the ESP32-S3 radio has no 5 GHz.)
//!
//! After the board prints "Got IP: a.b.c.d/24" test the server from a PC:
//!   nc <ip> 7777        (anything you type is echoed back)
//!
//! Architecture (all cooperative Embassy tasks on one executor):
//!   main        - initialises hardware, radio and network stack, spawns tasks
//!   connection  - keeps the Wi-Fi link up (connect / wait for disconnect / retry)
//!   net_task    - runs the embassy-net stack (smoltcp) forever
//!   echo_task   - listens on TCP port 7777 and echoes data
//! Wi-Fi uses ADC2-free pins only: never read ADC2 pins (GPIO11..20) while Wi-Fi is on.
//! API level: esp-radio 0.18.0 / esp-rtos 0.3.0 / embassy-net 0.9 (companion crates, not esp-hal).
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::{Runner, StackResources, tcp::TcpSocket};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock, interrupt::software::SoftwareInterruptControl, ram, rng::Rng,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::{
    Config, ControllerConfig, Interface, WifiController, scan::ScanConfig, sta::StationConfig,
};
use log::{info, warn};

esp_bootloader_esp_idf::esp_app_desc!();

// Allocate a value in a `static` and hand out a `&'static mut` to it.
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

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // The radio firmware needs a heap: one region in reclaimed IRAM, one in DRAM.
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    // esp-rtos provides the scheduler + Embassy time driver. It needs one hardware
    // timer and one software interrupt.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    if SSID == "YOUR_SSID" {
        warn!("SSID/PASSWORD not set at build time - set them as environment variables");
    }

    let station = Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into()),
    );
    let (mut controller, interfaces) = esp_radio::wifi::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(station),
    )
    .unwrap();

    // IP configuration from the router's DHCP server.
    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    let (stack, runner) = embassy_net::new(
        interfaces.station,
        net_config,
        mk_static!(StackResources<4>, StackResources::<4>::new()),
        seed,
    );

    info!("scanning for access points ...");
    let aps = controller
        .scan_async(&ScanConfig::default().with_max(10))
        .await
        .unwrap();
    for ap in aps {
        info!("  AP: {:?}", ap);
    }

    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    stack.wait_config_up().await; // resolves once DHCP has assigned an address
    if let Some(cfg) = stack.config_v4() {
        info!("Got IP: {}", cfg.address);
    }

    spawner.spawn(echo_task(stack).unwrap());

    loop {
        Timer::after(Duration::from_secs(10)).await;
        info!("link up: {}", stack.is_link_up());
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        info!("connecting to Wi-Fi ...");
        match controller.connect_async().await {
            Ok(info) => {
                info!("connected: {:?}", info);
                // Park here until the access point drops us, then retry.
                let reason = controller.wait_for_disconnect_async().await.ok();
                warn!("disconnected: {:?}", reason);
            }
            Err(e) => warn!("connect failed: {:?}", e),
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn echo_task(stack: embassy_net::Stack<'static>) {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];
    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(Duration::from_secs(30)));
        info!("echo: waiting for a client on port 7777");
        if socket.accept(7777).await.is_err() {
            warn!("echo: accept failed");
            continue;
        }
        info!("echo: client connected");
        let mut buf = [0u8; 128];
        loop {
            match socket.read(&mut buf).await {
                Ok(0) => break, // peer closed the connection
                Ok(n) => {
                    if socket.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        socket.close();
        info!("echo: client gone");
    }
}
