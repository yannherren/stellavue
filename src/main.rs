mod stepper;
mod web;
mod wifi;

use crate::web::protocol::CallbackHandler;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use std::sync::{Arc, Mutex};

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

    let stepper_clone = stepper.clone();
    let server_handler = CallbackHandler {
        move_constant: move |direction, steps_per_second| {
            stepper_clone
                .lock()
                .unwrap()
                .move_constant(direction, steps_per_second)
        },
        set_tracking: move |enable| (),
    };

    let mut wifi = wifi::WifiConnection::new(peripherals.modem, sys_loop, Some(nvs));
    wifi.connect()?;

    let _server = web::server::WebServer::new(server_handler);

    loop {
        FreeRtos::delay_ms(10);
    }
}
