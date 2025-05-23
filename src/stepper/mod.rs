use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_svc::timer::{EspTimer, EspTimerService};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::info;

const ROD_PITCH_MM: f32 = 1.25;
const STEPS_PER_ROTATION: u32 = 3600;

pub enum StepperDirection {
    UP,
    DOWN,
}

pub struct Stepper<STATE, D, S> where D: OutputPin, S: OutputPin {
    state: STATE,
    dir_pin: Arc<Mutex<PinDriver<'static, D, Output>>>,
    step_pin: Arc<Mutex<PinDriver<'static, S, Output>>>,
    steps: u32,
    tracking: bool,
    steps_per_second: Arc<Mutex<f32>>,
    direction: Arc<Mutex<StepperDirection>>,
    timer: Option<EspTimer<'static>>
}

pub struct Off;

pub struct On;

impl<D, S> Stepper<Off, D, S> where D: OutputPin, S: OutputPin {

    pub fn new(dir: PinDriver<'static, D, Output>, step: PinDriver<'static, S, Output>) -> Self {
        Stepper {
            state: Off,
            dir_pin: Arc::new(Mutex::new(dir)),
            step_pin: Arc::new(Mutex::new(step)),
            steps: 0,
            tracking: false,
            steps_per_second: Arc::new(Mutex::new(0.0)),
            direction: Arc::new(Mutex::new(StepperDirection::UP)),
            timer: None,
        }
    }

    pub fn switch_on(&self) -> Stepper<On, D, S> {
        let acc = Arc::new(Mutex::new(0.0));
        let timer_service = EspTimerService::new().unwrap();
        let callback_timer = {
            let mut steps_per_second_clone = self.steps_per_second.clone();
            let mut step_pin_clone = self.step_pin.clone();
            let mut acc_clone = acc.clone();
            timer_service.timer(move || {
                let mut step = step_pin_clone.lock().unwrap();
                let mut acc = acc_clone.lock().unwrap();
                let steps_per_second = steps_per_second_clone.lock().unwrap();
                if *steps_per_second == 0.0 { return }

                *acc += *steps_per_second / 10_000.0;

                if *acc >= 1.0 {
                    *acc -= 1.0;
                    step.set_high().unwrap();
                    Ets::delay_us(5);
                    step.set_low().unwrap();
                }
            }).unwrap()
        };

        callback_timer.every(Duration::from_micros(100)).unwrap();

        Stepper {
            state: On,
            dir_pin: self.dir_pin.clone(),
            step_pin: self.step_pin.clone(),
            steps: self.steps,
            tracking: false,
            direction: self.direction.clone(),
            steps_per_second: self.steps_per_second.clone(),
            timer: Some(callback_timer)
        }
    }
}

impl<D, S> Stepper<On, D, S> where D: OutputPin, S: OutputPin {

    //
    pub fn move_constant(&self, direction: StepperDirection, speed: u16) {
        let mut steps_per_second_clone = self.steps_per_second.clone();
        let mut steps_per_second = steps_per_second_clone.lock().unwrap();
        *steps_per_second = speed as f32;
        info!("{}", *steps_per_second);
    }
    // pub fn start_tracking(&mut self) {
    //     self.tracking = true;
    // }
    //
    // pub fn stop_tracking(&mut self) {
    //     self.tracking = false;
    // }
    //
    // pub fn get_next_movement_tick_delay(&self) -> Option<u32> {
    //     if self.tracking {
    //         Some(STEPS_PER_ROTATION)
    //     } else {
    //         Some(8_000_000 / self.steps_per_second as u32)
    //     }
    // }
    //
    // fn get_height_mm(steps: u32) -> f32 {
    //     let rotations: u32 = steps / STEPS_PER_ROTATION;
    //     rotations as f32 * ROD_PITCH_MM
    // }
    //
    // fn get_steps_per_second(&self, distance_mm: f32) -> f32 {
    //     let rotations: u32 = self.steps / STEPS_PER_ROTATION;
    //     rotations as f32 * ROD_PITCH_MM
    // }
}
