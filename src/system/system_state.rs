#[derive(PartialEq)]
pub enum SystemState {
    Tracking,
    Calibrating,
    Moving,
    Idle
}

impl SystemState {
    pub fn new() -> Self {
        SystemState::Idle
    }

    pub fn transition(&mut self, new_state: SystemState) -> bool {
        if self.transition_allowed(&new_state) {
            *self = new_state;
            return true
        }
        false
    }

    fn transition_allowed(&self, new_state: &SystemState) -> bool {
        Self::get_allowed_transitions(self).contains(new_state)
    }

    fn get_allowed_transitions(state: &SystemState) -> Vec<SystemState> {
        match state {
            SystemState::Tracking => Vec::from([SystemState::Idle]),
            SystemState::Calibrating => Vec::from([]),
            SystemState::Moving => Vec::from([SystemState::Idle, SystemState::Moving]),
            SystemState::Idle => Vec::from([SystemState::Moving, SystemState::Calibrating, SystemState::Tracking])
        }
    }
}