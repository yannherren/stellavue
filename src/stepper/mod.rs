mod rotation_state;
mod stepper_event;

use crate::stepper::rotation_state::RotationState;
use esp_idf_svc::eventloop::{EspEventDeserializer, EspEventLoop, EspEventSerializer, EspEventSource, System};
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_svc::timer::{EspTimer, EspTimerService};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use esp_idf_svc::hal::delay;
pub use crate::stepper::stepper_event::StepperEvent;

const ROD_PITCH_MM: f32 = 1.25;
const STEPS_PER_ROTATION: u32 = 3600;

#[derive(PartialEq)]
pub enum StepperDirection {
    UP,
    DOWN,
}

pub struct Stepper<STATE, D, S> where D: OutputPin, S: OutputPin {
    state: STATE,
    tracking: Arc<Mutex<bool>>,
    dir_pin: Arc<Mutex<PinDriver<'static, D, Output>>>,
    step_pin: Arc<Mutex<PinDriver<'static, S, Output>>>,
    rotation_state: Arc<Mutex<RotationState>>,
    direction: Arc<Mutex<StepperDirection>>,
    timer: Option<EspTimer<'static>>,
    sys_loop: EspEventLoop<System>
}

pub struct Off;

pub struct On;

impl<D, S> Stepper<Off, D, S> where D: OutputPin, S: OutputPin {

    pub fn new(dir: PinDriver<'static, D, Output>, step: PinDriver<'static, S, Output>, sys_loop: EspEventLoop<System>) -> Self {
        Stepper {
            state: Off,
            tracking: Arc::new(Mutex::new(false)),
            dir_pin: Arc::new(Mutex::new(dir)),
            step_pin: Arc::new(Mutex::new(step)),
            rotation_state: Arc::new(Mutex::new(RotationState::new())),
            direction: Arc::new(Mutex::new(StepperDirection::UP)),
            timer: None,
            sys_loop
        }
    }

    pub fn switch_on(&self) -> Stepper<On, D, S> {
        let acc: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
        let timer_service = EspTimerService::new().unwrap();

        let callback_timer = {
            let tracking_active = self.tracking.clone();
            let current_direction = self.direction.clone();
            let mut rotation_state_clone = self.rotation_state.clone();
            let mut step_pin_clone = self.step_pin.clone();
            let mut acc_clone = acc.clone();
            let sys_loop_clone = self.sys_loop.clone();

            timer_service.timer(move || Self::timer_tick(
                &current_direction,
                &tracking_active,
                &mut step_pin_clone,
                &mut rotation_state_clone,
                &mut acc_clone,
                &sys_loop_clone
            )).unwrap()
        };

        // let sys_loop_clone = self.sys_loop.clone();
        // let test_timer = timer_service.timer(move || {
        //     info!("in_timer");
        //     sys_loop_clone.post::<CustomEvent>(&CustomEvent::Start, delay::BLOCK).unwrap();
        // }).unwrap();
        // info!("start_timer");
        // test_timer.every(Duration::from_secs(2)).unwrap();
        // core::mem::forget(test_timer);


        Stepper {
            state: On,
            tracking: self.tracking.clone(),
            dir_pin: self.dir_pin.clone(),
            step_pin: self.step_pin.clone(),
            direction: self.direction.clone(),
            rotation_state: self.rotation_state.clone(),
            timer: Some(callback_timer),
            sys_loop: self.sys_loop.clone()
        }
    }

    fn timer_tick(
        direction: &Arc<Mutex<StepperDirection>>,
        tracking_active: &Arc<Mutex<bool>>,
        step_pin: &mut Arc<Mutex<PinDriver<'static, S, Output>>>,
        rotation_state: &mut Arc<Mutex<RotationState>>,
        acc: &mut Arc<Mutex<f32>>,
        sys_loop: &EspEventLoop<System>
    ) {
        let tracking = tracking_active.lock().unwrap();
        let mut step = step_pin.lock().unwrap();
        let mut acc = acc.lock().unwrap();
        let mut rotation_state = rotation_state.lock().unwrap();
        let steps_per_second = rotation_state.steps_per_second;
        if steps_per_second == 0 { return }

        *acc += steps_per_second as f32 / 10_000.0;

        if *acc >= 1.0 {
            *acc -= 1.0;
            step.set_high().unwrap();
            Ets::delay_us(5);
            step.set_low().unwrap();

            let (rotations, _offset) = rotation_state.get_rotation();

            let direction = direction.lock().unwrap();

            if rotation_state.max_reached() || rotation_state.min_reached() {

            }

            let (modified_rotations, modified_offset) =
                if *direction == StepperDirection::UP {
                    rotation_state.increment_step()
                } else {
                    rotation_state.decrement_step()
                };

            if rotations != modified_rotations {
                sys_loop.post::<StepperEvent>(&StepperEvent::RotationComplete(modified_rotations), delay::BLOCK).unwrap();
            }
            if *tracking {
                // Only post steps when tracking since the tracking speed is slow
                // Otherwise too many events are fired
                sys_loop.post::<StepperEvent>(&StepperEvent::StepComplete(modified_rotations, modified_offset), delay::BLOCK).unwrap();
                rotation_state.update_speed();
            }
        }
    }
}

impl<D, S> Stepper<On, D, S> where D: OutputPin, S: OutputPin {

    pub fn move_constant(&mut self, direction: StepperDirection, speed: u16) {
        self.sys_loop.post::<StepperEvent>(&StepperEvent::MovementStartUp, delay::BLOCK).unwrap();
        self.stop_movement();
        self.set_direction(direction);
        {
            let mut rotation_state = self.rotation_state.lock().unwrap();
            rotation_state.set_speed(speed);
        }
        self.start_timer();
    }

    pub fn start_tracking(&mut self) {
        self.sys_loop.post::<StepperEvent>(&StepperEvent::TrackingStart, delay::BLOCK).unwrap();
        self.stop_movement();
        {
            let mut tracking_active = self.tracking.lock().unwrap();
            *tracking_active = true;
        }
        self.rotation_state.lock().unwrap().update_speed();
        self.start_timer();
    }

    pub fn stop_movement(&mut self) {
        self.sys_loop.post::<StepperEvent>(&StepperEvent::MovementStop, delay::BLOCK).unwrap();
        if self.timer_active() {
            {
                let mut tracking_active = self.tracking.lock().unwrap();
                *tracking_active = false;
            }
            self.stop_timer();
        }
    }

    fn set_direction(&mut self, new_direction: StepperDirection) {
        let mut direction = self.direction.lock().unwrap();
        let mut direction_pin = self.dir_pin.lock().unwrap();
        *direction = new_direction;
        if *direction == StepperDirection::UP {
            (*direction_pin).set_high().unwrap();
        } else {
            (*direction_pin).set_low().unwrap();
        }
    }

    fn start_timer(&mut self) {
        if let Some(timer) = &self.timer {
            timer.every(Duration::from_micros(100)).unwrap();
        }
    }

    fn stop_timer(&mut self) {
        if let Some(timer) = &self.timer {
            timer.cancel().unwrap();
        }
    }

    fn timer_active(&mut self) -> bool {
        if let Some(timer) = &self.timer {
            return timer.is_scheduled().unwrap();
        }
        false
    }
}
