mod camera;
mod stepper;
mod system;
mod web;
mod wifi;

use crate::camera::CameraDriver;
use crate::system::system_state::SystemState;
use crate::web::server::CallbackHandler;
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

    let state = Arc::new(Mutex::new(SystemState::new()));

    let step = PinDriver::output(peripherals.pins.gpio3)?;
    let dir = PinDriver::output(peripherals.pins.gpio4)?;
    let shutter = PinDriver::output(peripherals.pins.gpio5)?;

    let camera_driver = CameraDriver::new(shutter, sys_loop.clone());

    let stepper = stepper::Stepper::new(dir, step, sys_loop.clone()); // TODO: add stop callback!
    let mut stepper = Arc::new(Mutex::new(stepper.switch_on()));
    stepper.lock().unwrap().start_calibration();
    {
        let mut start_state = state.lock().unwrap();
        *start_state = SystemState::Calibrating;
    }

    let mut limit_switch = PinDriver::input(peripherals.pins.gpio2)?;
    limit_switch.set_pull(Pull::Up)?;
    limit_switch.set_interrupt_type(InterruptType::PosEdge)?;

    unsafe {
        limit_switch.subscribe(move || {
            notifier.notify_and_yield(NonZeroU32::new(1).unwrap());
        })?;
    }

    let stepper_move = stepper.clone();
    let stepper_calibrate = stepper.clone();
    let stepper_track = stepper.clone();
    let stepper_stop = stepper.clone();

    let state_for_move = state.clone();
    let state_for_calibrate = state.clone();
    let state_for_track = state.clone();
    let state_for_stop = state.clone();
    let state_for_read = state.clone();

    let camera_for_test_capture = camera_driver.clone();
    let camera_for_start = camera_driver.clone();
    let camera_for_stop = camera_driver.clone();

    let server_handler = CallbackHandler {
        move_constant: Box::new(move |direction, steps_per_second| {
            let mut state = state_for_move.lock().unwrap();
            if state.transition(SystemState::Moving) {
                stepper_move
                    .lock()
                    .unwrap()
                    .move_constant(direction, steps_per_second)
            }
        }),
        start_calibration: Box::new(move || {
            let mut state = state_for_calibrate.lock().unwrap();
            if state.transition(SystemState::Calibrating) {
                stepper_calibrate.lock().unwrap().start_calibration()
            }
        }),
        set_tracking: Box::new(move |enable| {
            let mut state = state_for_track.lock().unwrap();
            if enable && state.transition(SystemState::Tracking) {
                stepper_track.lock().unwrap().set_tracking(true);
            } else if state.transition(SystemState::Idle) {
                stepper_track.lock().unwrap().set_tracking(false);
            }
        }),
        stop: Box::new(move || {
            let mut state = state_for_stop.lock().unwrap();
            if state.transition(SystemState::Idle) {
                stepper_stop.lock().unwrap().stop_movement(true);
            }
        }),
        get_state: Box::new(move || {
            let state = state_for_read.lock().unwrap();
            *state
        }),
        trigger_test_capture: Box::new(move || {
            camera_for_test_capture.lock().unwrap().capture();
        }),
        start_auto_capture: Box::new(move |interval| {
            CameraDriver::start_auto_capture(&camera_for_start, interval as u64);
        }),
        stop_auto_capture: Box::new(move || {
            CameraDriver::stop_auto_capture(&camera_for_stop);
        }),
    };

    let mut wifi = wifi::WifiConnection::new(peripherals.modem, sys_loop.clone(), Some(nvs));
    wifi.connect()?;

    let _server = web::server::WebServer::new(server_handler, sys_loop.clone());
    loop {
        limit_switch.enable_interrupt()?;
        notification.wait(esp_idf_svc::hal::delay::BLOCK);
        let stepper_clone = stepper.clone();
        stepper_clone.lock().unwrap().end_calibration();
        state.lock().unwrap().transition(SystemState::Idle);
    }
}
