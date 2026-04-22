mod rotation_state;

use crate::stepper::rotation_state::RotationState;
pub use crate::stepper::rotation_state::{STEPS_PER_ROTATION, MAX_ROTATIONS};
use crate::stepper::StepperDirection::UP;
use crate::system::system_event::SystemEvent;
use esp_idf_svc::eventloop::{
    EspEventDeserializer, EspEventLoop, EspEventSerializer, EspEventSource, System,
};
use esp_idf_svc::hal::delay;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_svc::timer::{EspTimer, EspTimerService};
use log::info;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const ROD_PITCH_MM: f32 = 1.25;

#[derive(PartialEq)]
pub enum StepperDirection {
    UP,
    DOWN,
}

pub struct Stepper<STATE, D, S>
where
    D: OutputPin,
    S: OutputPin,
{
    state: STATE,
    tracking: Arc<Mutex<bool>>,
    calibrating: Arc<Mutex<bool>>,
    dir_pin: Arc<Mutex<PinDriver<'static, D, Output>>>,
    step_pin: Arc<Mutex<PinDriver<'static, S, Output>>>,
    rotation_state: Arc<Mutex<RotationState>>,
    direction: Arc<Mutex<StepperDirection>>,
    timer: Arc<Mutex<Option<EspTimer<'static>>>>,
    sys_loop: EspEventLoop<System>,
}

pub struct Off;

pub struct On;

impl<D, S> Stepper<Off, D, S>
where
    D: OutputPin,
    S: OutputPin,
{
    pub fn new(
        dir: PinDriver<'static, D, Output>,
        step: PinDriver<'static, S, Output>,
        sys_loop: EspEventLoop<System>,
    ) -> Self {
        Stepper {
            state: Off,
            tracking: Arc::new(Mutex::new(false)),
            calibrating: Arc::new(Mutex::new(false)),
            dir_pin: Arc::new(Mutex::new(dir)),
            step_pin: Arc::new(Mutex::new(step)),
            rotation_state: Arc::new(Mutex::new(RotationState::new())),
            direction: Arc::new(Mutex::new(StepperDirection::UP)),
            timer: Arc::new(Mutex::new(None)),
            sys_loop,
        }
    }

    pub fn switch_on(&self) -> Stepper<On, D, S> {
        let acc: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
        let timer_service = EspTimerService::new().unwrap();

        let callback_timer = {
            let tracking_active = self.tracking.clone();
            let calibration_active = self.calibrating.clone();
            let current_direction = self.direction.clone();
            let mut rotation_state_clone = self.rotation_state.clone();
            let mut step_pin_clone = self.step_pin.clone();
            let mut acc_clone = acc.clone();
            let mut timer_clone = self.timer.clone();
            let sys_loop_clone = self.sys_loop.clone();

            timer_service
                .timer(move || {
                    Self::timer_tick(
                        &mut timer_clone,
                        &current_direction,
                        &tracking_active,
                        &calibration_active,
                        &mut step_pin_clone,
                        &mut rotation_state_clone,
                        &mut acc_clone,
                        &sys_loop_clone,
                    )
                })
                .unwrap()
        };

        // let sys_loop_clone = self.sys_loop.clone();
        // let test_timer = timer_service.timer(move || {
        //     info!("in_timer");
        //     sys_loop_clone.post::<CustomEvent>(&CustomEvent::Start, delay::BLOCK).unwrap();
        // }).unwrap();
        // info!("start_timer");
        // test_timer.every(Duration::from_secs(2)).unwrap();
        // core::mem::forget(test_timer);

        let mut timer = self.timer.lock().unwrap();
        *timer = Some(callback_timer);

        Stepper {
            state: On,
            tracking: self.tracking.clone(),
            calibrating: self.calibrating.clone(),
            dir_pin: self.dir_pin.clone(),
            step_pin: self.step_pin.clone(),
            direction: self.direction.clone(),
            rotation_state: self.rotation_state.clone(),
            timer: self.timer.clone(),
            sys_loop: self.sys_loop.clone(),
        }
    }

    fn timer_tick(
        timer: &mut Arc<Mutex<Option<EspTimer>>>,
        direction: &Arc<Mutex<StepperDirection>>,
        tracking_active: &Arc<Mutex<bool>>,
        calibration_active: &Arc<Mutex<bool>>,
        step_pin: &mut Arc<Mutex<PinDriver<'static, S, Output>>>,
        rotation_state: &mut Arc<Mutex<RotationState>>,
        acc: &mut Arc<Mutex<f32>>,
        sys_loop: &EspEventLoop<System>,
    ) {
        let tracking = { *(tracking_active.lock().unwrap()) }; // important: read and release to prevent deadlock
        let mut step = step_pin.lock().unwrap();
        let mut acc = acc.lock().unwrap();
        let mut rotation_state = rotation_state.lock().unwrap();
        let steps_per_second = rotation_state.steps_per_second;
        if steps_per_second == 0 {
            return;
        }

        *acc += steps_per_second as f32 / 10_000.0;

        if *acc >= 1.0 {
            *acc -= 1.0;
            step.set_high().unwrap();
            Ets::delay_us(5);
            step.set_low().unwrap();

            let (rotations, _offset) = rotation_state.get_rotation();

            let calibrating = calibration_active.lock().unwrap();

            if *calibrating {
                return;
            }

            let direction = direction.lock().unwrap();

            let (modified_rotations, modified_offset) = if *direction == StepperDirection::UP {
                rotation_state.increment_step()
            } else {
                rotation_state.decrement_step()
            };

            if rotation_state.max_reached() || rotation_state.min_reached() {
                info!("max reached: {:?}", tracking);
                let timer = timer.lock().unwrap();
                let mut tracking_active = tracking_active.lock().unwrap();
                if let Some(ref timer) = *timer {
                    sys_loop
                        .post::<SystemEvent>(&SystemEvent::MovementStop, delay::BLOCK)
                        .unwrap();
                    (*timer).cancel().unwrap();
                    *tracking_active = false;
                    return;
                }
                return;
            }

            if rotations != modified_rotations {
                sys_loop
                    .post::<SystemEvent>(
                        &SystemEvent::RotationComplete(modified_rotations),
                        delay::BLOCK,
                    )
                    .unwrap();
            }
            if tracking {
                // Only post steps when tracking since the tracking speed is slow
                // Otherwise too many events are fired
                sys_loop
                    .post::<SystemEvent>(
                        &SystemEvent::StepComplete(modified_rotations, modified_offset),
                        delay::BLOCK,
                    )
                    .unwrap();
                rotation_state.update_speed_from_config();
            }
        }
    }
}

impl<D, S> Stepper<On, D, S>
where
    D: OutputPin,
    S: OutputPin,
{
    pub fn move_constant(&mut self, direction: StepperDirection, speed: u16) {
        self.sys_loop
            .post::<SystemEvent>(
                &SystemEvent::MovementStarted(if direction == UP { 1 } else { 0 }, speed),
                delay::BLOCK,
            )
            .unwrap();

        self.stop_movement(false);
        self.set_direction(direction);
        {
            let mut rotation_state = self.rotation_state.lock().unwrap();
            rotation_state.set_speed(speed);
        }
        self.start_timer();
    }

    pub fn set_tracking(&mut self, enabled: bool) {
        if enabled {
            self.start_tracking()
        } else {
            self.stop_movement(true)
        }
    }

    pub fn start_tracking(&mut self) {
        self.sys_loop
            .post::<SystemEvent>(&SystemEvent::TrackingStart, delay::BLOCK)
            .unwrap();
        self.stop_movement(false);
        self.set_direction(StepperDirection::UP);
        {
            let mut tracking_active = self.tracking.lock().unwrap();
            *tracking_active = true;
        }
        self.rotation_state
            .lock()
            .unwrap()
            .update_speed_from_config();
        info!("start timer!!!");
        self.start_timer();
    }

    pub fn stop_movement(&mut self, post_event: bool) {
        if post_event {
            self.sys_loop
                .post::<SystemEvent>(&SystemEvent::MovementStop, delay::BLOCK)
                .unwrap();
        }
        if self.timer_active() {
            {
                let mut tracking_active = self.tracking.lock().unwrap();
                *tracking_active = false;
            }
            self.stop_timer();
        }
    }

    pub fn start_calibration(&mut self) {
        self.sys_loop
            .post::<SystemEvent>(&SystemEvent::CalibrationStarted, delay::BLOCK)
            .unwrap();
        {
            let mut calibrating = self.calibrating.lock().unwrap();
            *calibrating = true;
        }
        self.move_constant(StepperDirection::DOWN, STEPS_PER_ROTATION); // one rotation per second
    }

    pub fn end_calibration(&mut self) {
        self.stop_movement(true);
        self.rotation_state.lock().unwrap().reset();
        let mut calibrating = self.calibrating.lock().unwrap();
        *calibrating = false;
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
        let timer = self.timer.lock().unwrap();
        if let Some(ref timer) = *timer {
            timer.every(Duration::from_micros(100)).unwrap();
        }
    }

    fn stop_timer(&mut self) {
        let timer = self.timer.lock().unwrap();
        if let Some(ref timer) = *timer {
            timer.cancel().unwrap();
        }
    }

    fn timer_active(&mut self) -> bool {
        let timer = self.timer.lock().unwrap();
        if let Some(ref timer) = *timer {
            return timer.is_scheduled().unwrap();
        }
        false
    }
}
