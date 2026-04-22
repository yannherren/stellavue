use crate::stepper::StepperDirection;
use crate::system::system_event::SystemEvent;
use crate::web;
use crate::web::protocol::{event_to_response, parse_response, Command};
use esp_idf_svc::eventloop::{EspEventLoop, EspSubscription, System};
use esp_idf_svc::http::server::ws::{EspHttpWsConnection, EspHttpWsDetachedSender};
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::sys::{EspError, ESP_ERR_INVALID_SIZE};
use esp_idf_svc::ws::FrameType;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

static INDEX_HTML: &str = include_str!("webapp/index.html");
static INDEX_CSS: &str = include_str!("webapp/stylesheet.css");

static INDEX_JS: &str = include_str!("webapp/index.js");

const COMMAND_LEN: usize = 4;

pub struct WebServer {
    http_server: EspHttpServer<'static>,
    sys_loop: EspEventLoop<System>,
    last_event: Arc<Mutex<SystemEvent>>,
}

pub struct CallbackHandler {
    pub move_constant: Box<dyn Fn(StepperDirection, u16) + Send + Sync>,
    pub start_calibration: Box<dyn Fn() + Send + Sync>,
    pub set_tracking: Box<dyn Fn(bool) + Send + Sync>,
}

#[derive(Ord, Eq, PartialOrd, PartialEq)]
enum SubscriptionType {
    GlobalEventSubscriber,
    SessionEventSubscriber(i32),
}

impl WebServer {
    pub fn new(handler: CallbackHandler, sys_loop: EspEventLoop<System>) -> Self {
        let sys_loop_clone = sys_loop.clone();
        let last_event = Arc::new(Mutex::new(SystemEvent::CalibrationStarted));

        let mut server = WebServer::create_web_server();
        let mut sub = Mutex::new(BTreeMap::<SubscriptionType, EspSubscription<System>>::new());

        Self::register_static_resource(&mut server, "/", INDEX_HTML, "text/html");
        Self::register_static_resource(&mut server, "/stylesheet.css", INDEX_CSS, "text/css");
        Self::register_static_resource(&mut server, "/index.js", INDEX_JS, "text/javascript");

        let last_event_clone = last_event.clone();
        let last_event_subscription = sys_loop
            .subscribe::<SystemEvent, _>(move |event| {
                let mut last_event = last_event_clone.lock().unwrap();
                *last_event = event;
            })
            .unwrap();
        sub.lock().unwrap().insert(
            SubscriptionType::GlobalEventSubscriber,
            last_event_subscription,
        );

        let last_event_clone = last_event.clone();

        server
            .ws_handler("/ws/tracker", move |ws| {
                if ws.is_closed() {
                    let session_id = ws.session();
                    Self::deregister_subscription(&sub, session_id);
                    return Ok(());
                }

                if ws.is_new() {
                    Self::register_subscription(&*ws, &sub, &sys_loop_clone);
                    return Ok(());
                }

                let (_frame_type, len) = ws.recv(&mut []).unwrap();
                if len != COMMAND_LEN {
                    return Err(EspError::from_infallible::<ESP_ERR_INVALID_SIZE>());
                }
                let mut buf = [0; COMMAND_LEN];
                ws.recv(buf.as_mut())?;

                Self::handle_command(&handler, &last_event_clone, &ws, buf);

                return Ok::<(), EspError>(());
            })
            .unwrap();

        WebServer {
            http_server: server,
            sys_loop,
            last_event,
        }
    }

    fn handle_command(
        handler: &CallbackHandler,
        last_event: &Arc<Mutex<SystemEvent>>,
        ws: &EspHttpWsConnection,
        buffer: [u8; COMMAND_LEN]
    ) {
        let command_bits: u32 = ((buffer[0] as u32) << 24)
            + ((buffer[1] as u32) << 16)
            + ((buffer[2] as u32) << 8)
            + buffer[3] as u32;

        let command = web::protocol::map_command(command_bits);

        match command {
            Command::StartCalibration => (handler.start_calibration)(),
            Command::MoveConstant(direction, speed) => {
                (handler.move_constant)(direction, speed)
            }
            Command::SetTracking(enable) => (handler.set_tracking)(enable),
            Command::RepeatLastEvent => {
                let mut sender = ws.create_detached_sender().unwrap();
                let last_event_to_send = *last_event.lock().unwrap();
                Self::send_event_response(&mut sender, &last_event_to_send)
            }
            Command::Unknown => {}
        }
    }

    fn register_static_resource(
        server: &mut EspHttpServer,
        uri: &str,
        content: &'static str,
        content_type: &'static str,
    ) {
        server
            .fn_handler(uri, Method::Get, move |req| {
                let headers = [("Content-Type", content_type)];
                req.into_response(200, Some("OK"), &headers)?
                    .write_all(content.as_bytes())
                    .map(|_| ())
            })
            .unwrap();
    }

    fn register_subscription(
        ws: &EspHttpWsConnection,
        subscriptions: &Mutex<BTreeMap<SubscriptionType, EspSubscription<System>>>,
        sys_loop: &EspEventLoop<System>,
    ) {
        let mut subscriptions = subscriptions.lock().unwrap();

        let mut detached_sender = ws.create_detached_sender().unwrap();
        let subscription = sys_loop
            .subscribe::<SystemEvent, _>(move |event| {
                Self::send_event_response(&mut detached_sender, &event)
            })
            .unwrap();
        let stepper_key = SubscriptionType::SessionEventSubscriber(ws.session());
        subscriptions.insert(stepper_key, subscription);
    }

    fn deregister_subscription(
        subscriptions: &Mutex<BTreeMap<SubscriptionType, EspSubscription<System>>>,
        session_id: i32,
    ) {
        let mut subscriptions = subscriptions.lock().unwrap();

        let stepper_key = SubscriptionType::SessionEventSubscriber(session_id);
        if subscriptions.contains_key(&stepper_key) {
            subscriptions.remove(&stepper_key);
        }
    }

    fn send_event_response(detached_sender: &mut EspHttpWsDetachedSender, event: &SystemEvent) {
        if let Some(response) = event_to_response(*event) {
            let byte_response = parse_response(response);
            detached_sender
                .send(FrameType::Binary(false), &byte_response)
                .unwrap();
        }
    }

    fn create_web_server() -> EspHttpServer<'static> {
        let server_configuration = esp_idf_svc::http::server::Configuration {
            stack_size: 10240,
            ..Default::default()
        };
        let mut mdns = EspMdns::take().unwrap();
        mdns.set_hostname("stellavue").unwrap();
        mdns.add_service(
            Some("CONTROL"),
            "_http",
            "_tcp",
            server_configuration.http_port,
            &[],
        )
        .unwrap();
        core::mem::forget(mdns);

        EspHttpServer::new(&server_configuration).unwrap()
    }
}
