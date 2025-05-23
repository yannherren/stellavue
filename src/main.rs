mod stepper;
mod web;

use std::ffi::CStr;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::{Ets, FreeRtos};
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::timer::{config, TimerDriver};
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{esp_wifi_set_max_tx_power, EspError, ESP_ERR_INVALID_SIZE};
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::{AccessPointConfiguration, AuthMethod, BlockingWifi, EspWifi};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use esp_idf_svc::ws::FrameType;
use log::info;
use crate::stepper::StepperDirection;
use crate::web::protocol::CallbackHandler;

const COMMAND_MAX_LEN: usize = 2;

static INDEX_HTML: &str = include_str!("index.html");

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let step = PinDriver::output(peripherals.pins.gpio3)?;
    let dir = PinDriver::output(peripherals.pins.gpio4)?;

    let stepper = stepper::Stepper::new(dir, step);
    let mut stepper = Arc::new(Mutex::new(stepper.switch_on()));

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

    server.fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?
            .write_all(INDEX_HTML.as_bytes())
            .map(|_| ())
    })?;

    let stepper_clone = stepper.clone();
    server.ws_handler("/ws/tracker", move |ws| {

        if ws.is_new() || ws.is_closed() { return Ok(()) }



        let (_frame_type, len) = ws.recv(&mut []).unwrap();
        info!("Len: {}", len);
        if len > COMMAND_MAX_LEN { return Err(EspError::from_infallible::<ESP_ERR_INVALID_SIZE>())}
        let mut buf = [0; COMMAND_MAX_LEN];
        ws.recv(buf.as_mut())?;
        let command: u16 = ((buf[0] as u16) << 8) + buf[1] as u16;

        let handler = CallbackHandler {
            move_constant: |direction, steps_per_second| stepper_clone.lock().unwrap().move_constant(direction, steps_per_second),
            set_tracking: |enable| ()
        };

        web::protocol::map_command(handler, command);

        // let command = CStr::from_bytes_until_nul(&buf[..len]).unwrap().to_str().unwrap();
        info!("Command: {:?}", buf);
        // ws.send(FrameType::Text(true), command.as_bytes())?;

        return Ok::<(), EspError>(())
    })?;
    //
    // let stepper_clone = stepper.clone();
    // server.fn_handler("/control", Method::Post, move |req| {
    //     stepper_clone.lock().unwrap().start_movement();
    //     req.into_ok_response()?.write_all("Running!".as_bytes()).map(|_| ())
    // })?;

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
