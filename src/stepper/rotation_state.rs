use serde::Deserialize;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use log::info;

const ROTATION_CONFIG: &str = include_str!("rotation_speeds.json");

const STEPS_PER_ROTATION: u16 = 200 * 16; // 16x microstepping

#[derive(Serialize, Deserialize, Debug)]
struct RotationConfigData {
    rotations: u16,
    offset_steps: u16,
    steps_per_second: u16,
}

pub struct RotationState {
    pub steps_per_second: u16,
    pub rotations: u16,
    pub rotation_offset: u16, // use an offset to prevent overflows
    next_speed_rotations: u16,
    next_speed_rotation_offset: u16,
    next_speed_rotation_speed: u16,
    speed_config: [RotationConfigData; 9],
}

impl RotationState {
    pub fn new() -> Self {
        RotationState {
            steps_per_second: 0,
            rotations: 0,
            rotation_offset: 0,
            next_speed_rotations: 0,
            next_speed_rotation_offset: 0,
            next_speed_rotation_speed: 0,
            speed_config: serde_json::from_str(ROTATION_CONFIG).unwrap(),
        }
    }

    pub fn increment_step(&mut self) -> (u16, u16) {
        if self.rotation_offset >= STEPS_PER_ROTATION {
            self.rotation_offset = 0;
            self.rotations += 1;
        } else {
            self.rotation_offset += 1;
        }
        (self.rotations, self.rotation_offset)
    }

    pub fn decrement_step(&mut self) -> (u16, u16) {
        if self.rotation_offset <= 0 {
            self.rotation_offset = STEPS_PER_ROTATION - 1;
            self.rotations -= 1;
        } else {
            self.rotation_offset -= 1;
        }
        (self.rotations, self.rotation_offset)
    }

    pub fn update_speed(&mut self) {
        if self.next_speed_rotation_offset == 0 && self.next_speed_rotations == 0 {
            self.steps_per_second = self.speed_config[0].steps_per_second;
            self.next_speed_rotations = self.speed_config[1].rotations;
            self.next_speed_rotation_offset = self.speed_config[1].offset_steps;
            self.next_speed_rotation_speed = self.speed_config[1].steps_per_second;
        } else if self.rotations >= self.next_speed_rotations
            && self.rotation_offset >= self.next_speed_rotation_offset
        {
            self.steps_per_second = self.next_speed_rotation_speed;
            for data in self.speed_config.iter() {
                if data.rotations >= self.next_speed_rotations
                    && data.offset_steps > self.next_speed_rotation_offset
                {
                    self.next_speed_rotations = data.rotations;
                    self.next_speed_rotation_offset = data.offset_steps;
                    self.next_speed_rotation_speed = data.steps_per_second;
                    return;
                }
            }
            // TODO: what if ended
        }
    }

    pub fn set_speed(&mut self, steps_per_second: u16) {
        self.steps_per_second = steps_per_second;
    }

    pub fn get_rotation(&self) -> (u16, u16) {
        (self.rotations, self.rotation_offset)
    }
}
