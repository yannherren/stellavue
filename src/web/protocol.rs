use log::info;
use crate::stepper::StepperDirection;
use crate::stepper::StepperDirection::{DOWN, UP};

pub struct CallbackHandler<M, T>
where
    M: Fn(StepperDirection, u16),
    T: Fn(bool)
{
    pub move_constant: M,
    pub set_tracking: T
}

pub fn map_command<M, T>(handler: &CallbackHandler<M, T>, command: u16)
where
    M: Fn(StepperDirection, u16),
    T: Fn(bool)
{
    let command_type = 0b11 & command;
    let payload = command >> 2;
    match command_type {
        0b01 => {
            let direction = 0x1 & payload;
            let direction = if direction == 1 { UP } else { DOWN };
            let speed = payload >> 1;
            (handler.move_constant)(direction, speed);
        }
        0b10 => {
            let state = 0x1 & payload;
            let enable = if state == 1 { true } else { false };
            (handler.set_tracking)(enable);
        }
        _ => {}
    }
}