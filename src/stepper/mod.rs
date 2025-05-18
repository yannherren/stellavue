enum StepperDirection {
    UP,
    DOWN
}

pub struct Stepper<STATE> {
    state: STATE,
    rotations: u8,
    direction: StepperDirection
}

pub struct Off;

struct On;

impl Stepper<Off> {

    pub fn new() -> Self {
        Stepper {
            state: Off,
            rotations: 0,
            direction: StepperDirection::UP
        }
    }

    fn switch_on(self) -> Stepper<On> {
        Stepper { state: On, rotations: self.rotations, direction: self.direction }
    }
}