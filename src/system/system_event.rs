
use std::ffi::CStr;
use esp_idf_svc::eventloop::{EspEvent, EspEventDeserializer, EspEventPostData, EspEventSerializer, EspEventSource};

const NAME: &str = "SystemEvent\0";

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum SystemEvent {
    StepComplete(u16, u16),
    RotationComplete(u16),
    TrackingStart,
    MovementStartUp,
    MovementStartDown,
    MovementStop,
    RepeatLastEvent
}

unsafe impl EspEventSource for SystemEvent {
    #[allow(clippy::manual_c_str_literals)]
    fn source() -> Option<&'static CStr> {
        Some(CStr::from_bytes_with_nul(NAME.as_bytes()).unwrap())
    }
}

impl EspEventSerializer for SystemEvent {
    type Data<'a> = SystemEvent;

    fn serialize<F, R>(event: &Self::Data<'_>, f: F) -> R
    where
        F: FnOnce(&EspEventPostData) -> R,
    {
        f(&unsafe { EspEventPostData::new(Self::source().unwrap(), Self::event_id(), event) })
    }
}

impl EspEventDeserializer for SystemEvent {
    type Data<'a> = SystemEvent;

    fn deserialize<'a>(data: &EspEvent<'a>) -> Self::Data<'a> {
        *unsafe { data.as_payload::<SystemEvent>() }
    }
}