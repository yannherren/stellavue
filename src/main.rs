use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::{Ets, FreeRtos};
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::timer::{config, TimerDriver};
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{esp_wifi_set_max_tx_power, EspError};
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::{AccessPointConfiguration, AuthMethod, BlockingWifi, EspWifi};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static INDEX_HTML: &str = include_str!("index.html");

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let stepper_running = Arc::new(AtomicBool::new(false));

    let mut step = PinDriver::output(peripherals.pins.gpio3)?;
    let mut dir = PinDriver::output(peripherals.pins.gpio4)?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let wifi_config = wifi::Configuration::AccessPoint(AccessPointConfiguration {
        ssid: "Stellavue".try_into().unwrap(),
        ssid_hidden: false,
        channel: 11,
        auth_method: AuthMethod::WPA2Personal,
        password: "iseestars".try_into().unwrap(),
        max_connections: 1,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_config)?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    // ESP super mini antenna design is broken and only works with reduced tx power
    // see https://forum.arduino.cc/t/no-wifi-connect-with-esp32-c3-super-mini/1324046/12
    set_wifi_tx_power(12.0)?;

    let mut server = create_web_server();

    // log::info!("Hello, world!");

    dir.set_high()?;

    let timer_config = config::Config::new().auto_reload(true);
    let mut timer = TimerDriver::new(peripherals.timer00, &timer_config)?;

    server.fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?
            .write_all(INDEX_HTML.as_bytes())
            .map(|_| ())
    })?;

    let stepper_running_clone = stepper_running.clone();
    server.fn_handler("/control", Method::Post, move |req| {
        stepper_running_clone.store(true, Ordering::Relaxed);
        req.into_ok_response()?.write_all("Running!".as_bytes()).map(|_| ())
    })?;

    let stepper_running_clone = stepper_running.clone();
    unsafe {
        timer.subscribe(move || {
            if stepper_running_clone.load(Ordering::Relaxed) {
                step.set_high().unwrap();
                Ets::delay_us(10);
                step.set_low().unwrap();
            }
        })?;
    }

    timer.set_alarm(timer.tick_hz() / 3600)?;

    timer.enable_interrupt()?;
    timer.enable_alarm(true)?;
    timer.enable(true)?;

    loop {
        FreeRtos::delay_ms(10);
    }
}

fn create_web_server() -> EspHttpServer<'static> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    };

    EspHttpServer::new(&server_configuration).unwrap()
}

fn set_wifi_tx_power(dbm: f32) -> Result<(), EspError> {
    let power = (dbm * 4.0) as i8;
    let res = unsafe { esp_wifi_set_max_tx_power(power) };
    if res == 0 {
        Ok(())
    } else {
        Err(EspError::from(res).unwrap())
    }
}
