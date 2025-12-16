use log::info;
use crate::stepper::StepperDirection;
use crate::stepper::StepperDirection::{DOWN, UP};

pub enum ResponseType {
    AllMovementStopped,
    ConstantMovementStarted(u8, u16),
    TrackingStarted,
    CalibrationStarted,
    HeightChanged(u16),
}

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
            info!("{:?}", direction);
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

pub fn parse_response(response_type: ResponseType) -> [u8; 2] {
    let mut command = 0;

    match response_type {
        ResponseType::ConstantMovementStarted(direction, speed) => {
            command = speed << 3 + direction << 2 + 1
        }
        ResponseType::HeightChanged(percentage) => {
            command = percentage << 2 + 0b11
        }
        ResponseType::AllMovementStopped => {command = 0}
        ResponseType::TrackingStarted => {command = 0b10}
        ResponseType::CalibrationStarted => {command = 0b110}
    }

    command.to_be_bytes()
}

