use std::ffi::CStr;
use esp_idf_svc::eventloop::{EspEvent, EspEventDeserializer, EspEventPostData, EspEventSerializer, EspEventSource};

const NAME: &str = "STEPPER_EVENT\0";

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum StepperEvent {
    StepComplete(u16, u16),
    RotationComplete(u16),
    TrackingStart,
    MovementStartUp,
    MovementStartDown,
    MovementStop,
}

unsafe impl EspEventSource for StepperEvent {
    #[allow(clippy::manual_c_str_literals)]
    fn source() -> Option<&'static CStr> {
        Some(CStr::from_bytes_with_nul(NAME.as_bytes()).unwrap())
    }
}

impl EspEventSerializer for StepperEvent {
    type Data<'a> = StepperEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for StepperEvent {
    type Data<'a> = StepperEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        *unsafe { data.as_payload::<StepperEvent>() }
    }
}