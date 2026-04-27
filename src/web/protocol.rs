use log::info;
use crate::stepper::StepperDirection::{DOWN, UP};
use crate::stepper::{StepperDirection, MAX_ROTATIONS, STEPS_PER_ROTATION};
use crate::system::system_event::SystemEvent;
use crate::system::system_state::SystemState;

pub enum ResponseType {
    AllMovementStopped,
    ConstantMovementStarted(u8, u16),
    TrackingStarted,
    CalibrationStarted,
    HeightChanged(u16),
    SystemStateInfo(SystemState),
}

pub enum Command {
    StartCalibration,
    MoveConstant(StepperDirection, u16),
    SetTracking(bool),
    RequestStatus,
    Unknown,
}


pub struct SystemCallbackHandler {
    pub repeat_last_event: Box<dyn FnMut() + Send + 'static>,
}

pub fn map_command(
    command: u32,
) -> Command {
    let command_type = 0b1111 & command;
    let payload = command >> 4;
    match command_type {
        0b0000 => Command::StartCalibration,
        0b0001 => {
            let direction = 0x1 & payload;
            let direction = if direction == 1 { UP } else { DOWN };
            let speed = payload >> 1;
            Command::MoveConstant(direction, speed as u16)
        }
        0b0010 => {
            let state = 0x1 & payload;
            let enable = if state == 1 { true } else { false };
            Command::SetTracking(enable)
        }
        0b0011 => Command::RequestStatus,
        _ => Command::Unknown,
    }
}

pub fn event_to_response(event: SystemEvent) -> Option<ResponseType> {
    match event {
        SystemEvent::CalibrationStarted => Some(ResponseType::CalibrationStarted),
        SystemEvent::StepComplete(rotations, offset) => {
            let total_steps =
                u32::from(rotations) * u32::from(STEPS_PER_ROTATION) + u32::from(offset);
            let max_steps: u32 = u32::from(MAX_ROTATIONS) * u32::from(STEPS_PER_ROTATION);
            let percentage = total_steps * 100 / max_steps;
            info!("{:?}", percentage);
            Some(ResponseType::HeightChanged(percentage as u16))
        }
        SystemEvent::RotationComplete(rotations) => {
            let total_steps = u32::from(rotations) * u32::from(STEPS_PER_ROTATION);
            let max_steps: u32 = u32::from(MAX_ROTATIONS) * u32::from(STEPS_PER_ROTATION);
            let percentage = total_steps * 100 / max_steps;
            info!("{:?}", percentage);
            Some(ResponseType::HeightChanged(percentage as u16))
        }
        SystemEvent::TrackingStart => Some(ResponseType::TrackingStarted),
        SystemEvent::MovementStarted(direction, speed) => {
            Some(ResponseType::ConstantMovementStarted(direction, speed))
        }
        SystemEvent::MovementStop => Some(ResponseType::AllMovementStopped),
        SystemEvent::SystemStateInfo(state) => Some(ResponseType::SystemStateInfo(state)),
    }
}

pub fn parse_response(response_type: ResponseType) -> [u8; 4] {
    let mut command: u32 = 0;

    match response_type {
        ResponseType::ConstantMovementStarted(direction, speed) => {
            command = (speed << 5) as u32 + (direction << 4) as u32 + 0b0001
        }
        ResponseType::HeightChanged(percentage) => command = (percentage << 4) as u32 + 0b0011,
        ResponseType::AllMovementStopped => command = 0,
        ResponseType::TrackingStarted => command = 0b0010,
        ResponseType::CalibrationStarted => command = 0b0100,
        ResponseType::SystemStateInfo(state) => {
            command = 0b1111;
            match state {
                SystemState::Tracking => command = 0b110000 | command,
                SystemState::Calibrating => command = 0b010000 | command,
                SystemState::Moving => command = 0b100000 | command,
                SystemState::Idle => command = 0b000000 | command
            }
        },
    }

    command.to_be_bytes()
}
