mod stepper;
mod web;
mod wifi;
mod camera;

use std::cell::RefCell;
use crate::web::protocol::CallbackHandler;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::{Delay, Ets, FreeRtos};
use esp_idf_svc::hal::gpio::{Pin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use std::sync::{Arc, Mutex};
use embedded_hal_bus::i2c::RefCellDevice;
use esp_idf_svc::hal::i2c::*;
use esp_idf_svc::hal::prelude::*;
use log::info;
use mpu6050_dmp::address::Address;
use mpu6050_dmp::sensor::Mpu6050;
use crate::camera::CameraModule;

fn main() -> Result<(), EspError> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio6;
    let sdc = peripherals.pins.gpio7;
    // let shutter_pin = peripherals.pins.gpio5;

    let config = I2cConfig::new().baudrate(Hertz(100_000));
    let i2c0 = I2cDriver::new(i2c, sda, sdc, &config)?;
    let i2c_ref_cell = RefCell::new(i2c0);



    // let bus: &'static _ = shared_bus::new_std!(I2cDriver = i2c).unwrap();
    // let bus = shared_bus::BusManagerSimple::new();

    // let mut camera_module = CameraModule::new(RefCellDevice::new(&i2c_ref_cell), shutter_pin).unwrap();


    // let mut mpu6050 = Mpu6050::new(RefCellDevice::new(&i2c_ref_cell), Address::default()).unwrap();
    // let mut delay = Ets;
    // mpu6050.initialize_dmp(&mut delay).unwrap();

    let step = PinDriver::output(peripherals.pins.gpio3)?;
    let dir = PinDriver::output(peripherals.pins.gpio4)?;
    let mut shutter = PinDriver::output(peripherals.pins.gpio5)?;

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
        FreeRtos::delay_ms(10000);
        shutter.set_high()?;
        FreeRtos::delay_ms(1000);
        shutter.set_low()?;
        // let x = camera_module.get_acc().unwrap();
        // info!("{:?}", x)
        // let gyro = mpu6050.get_acc_angles().unwrap();
        //
        // let x = gyro.x * (180.0 / PI);
        // let y = gyro.y * (180.0 / PI);

        // let acc = mpu6050.accel().unwrap();
        // info!("{:?}", acc.x());
        // info!("Gyro: {x} {y}");

        // info!("Acc: {:?}", acc);
    }
}
