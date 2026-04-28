use crate::system::system_event::SystemEvent;
use embedded_hal::digital::OutputPin;
use esp_idf_svc::eventloop::{EspEventLoop, System};
use esp_idf_svc::hal::delay;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::timer::{EspTimer, EspTimerService};
use log::info;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct CameraDriver<P>
where
    P: OutputPin + Send + 'static,
{
    shutter_pin: P,
    timer: Option<EspTimer<'static>>,
    sys_loop: EspEventLoop<System>,
}

impl<P> CameraDriver<P>
where
    P: OutputPin + Send + 'static,
{
    pub fn new(shutter_pin: P, sys_loop: EspEventLoop<System>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(CameraDriver {
            shutter_pin,
            timer: None,
            sys_loop,
        }))
    }

    pub fn init(driver: &Arc<Mutex<Self>>) {
        let timer_service = EspTimerService::new().unwrap();
        let driver_clone = driver.clone();

        if let Ok(mut guard) = driver.lock() {
            let sys_loop_clone = guard.sys_loop.clone();
            let timer = Some({
                timer_service
                    .timer(move || Self::timer_tick(&driver_clone, &sys_loop_clone))
                    .unwrap()
            });
            guard.timer = timer;
        }
    }

    pub fn capture(&mut self) {
        self.shutter_pin.set_high().unwrap();
        FreeRtos::delay_ms(1000);
        self.shutter_pin.set_low().unwrap();
    }

    pub fn start_auto_capture(driver: &Arc<Mutex<Self>>, interval: u64) {
        if let Ok(mut guard) = driver.lock() {
            if let Some(ref timer) = guard.timer {
                info!("timer existent");
                guard
                    .sys_loop
                    .post::<SystemEvent>(&SystemEvent::AutoCaptureStarted, delay::BLOCK)
                    .unwrap();
                if timer.is_scheduled().unwrap() {
                    return;
                }
                timer.every(Duration::from_millis(interval)).unwrap();
            }
        }
    }

    pub fn stop_auto_capture(driver: &Arc<Mutex<Self>>) {
        if let Ok(mut guard) = driver.lock() {
            if let Some(ref timer) = guard.timer {
                guard
                    .sys_loop
                    .post::<SystemEvent>(&SystemEvent::AutoCaptureStopped, delay::BLOCK)
                    .unwrap();
                if !timer.is_scheduled().unwrap() {
                    return;
                }
                timer.cancel().unwrap();
            }
        }
    }

    fn timer_tick(driver: &Arc<Mutex<Self>>, sys_loop: &EspEventLoop<System>) {
        if let Ok(mut guard) = driver.lock() {
            guard.capture()
        }

        sys_loop
            .post::<SystemEvent>(&SystemEvent::ImageCaptured, delay::BLOCK)
            .unwrap();
    }
}
