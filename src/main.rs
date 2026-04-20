mod camera;
mod stepper;
mod web;
mod wifi;

use crate::web::protocol::CallbackHandler;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{InterruptType, Pin, PinDriver, Pull};
use esp_idf_svc::hal::i2c::*;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::task::notification::Notification;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let notification = Notification::new();
    let notifier = notification.notifier();

    let step = PinDriver::output(peripherals.pins.gpio3)?;
    let dir = PinDriver::output(peripherals.pins.gpio4)?;
    let mut shutter = PinDriver::output(peripherals.pins.gpio5)?;

    let stepper = stepper::Stepper::new(dir, step, sys_loop.clone());
    let mut stepper = Arc::new(Mutex::new(stepper.switch_on()));
    stepper.lock().unwrap().start_calibration();

    let mut limit_switch = PinDriver::input(peripherals.pins.gpio2)?;
    limit_switch.set_pull(Pull::Up)?;
    limit_switch.set_interrupt_type(InterruptType::PosEdge)?;

    unsafe {
        limit_switch.subscribe(move || {
            notifier.notify_and_yield(NonZeroU32::new(1).unwrap());
        })?;
    }

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

    let mut wifi = wifi::WifiConnection::new(peripherals.modem, sys_loop.clone(), Some(nvs));
    wifi.connect()?;

    let _server = web::server::WebServer::new(server_handler, sys_loop.clone());
    loop {
        limit_switch.enable_interrupt()?;
        notification.wait(esp_idf_svc::hal::delay::BLOCK);
        println!("Button pressed");

        let stepper_clone = stepper.clone();
        stepper_clone.lock().unwrap().end_calibration();
    }
}
