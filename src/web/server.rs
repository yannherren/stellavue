use crate::stepper::{StepperDirection, StepperEvent};
use crate::web;
use crate::web::protocol::CallbackHandler;
use crate::web::system_event::SystemEvent;
use esp_idf_svc::eventloop::{EspEventLoop, EspSubscription, System};
use esp_idf_svc::http::server::ws::EspHttpWsConnection;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::sys::{EspError, ESP_ERR_INVALID_SIZE};
use esp_idf_svc::ws::FrameType;
use std::collections::BTreeMap;
use std::sync::Mutex;

static INDEX_HTML: &str = include_str!("webapp/index.html");
static INDEX_CSS: &str = include_str!("webapp/stylesheet.css");

static INDEX_JS: &str = include_str!("webapp/index.js");

const COMMAND_LEN: usize = 2;

pub struct WebServer {
    http_server: EspHttpServer<'static>,
    sys_loop: EspEventLoop<System>,
}

impl WebServer {
    pub fn new<M, T>(handler: CallbackHandler<M, T>, sys_loop: EspEventLoop<System>) -> Self
    where
        M: Fn(StepperDirection, u16) + Send + Sync + 'static,
        T: Fn(bool) + Send + Sync + 'static,
    {
        let mut server = WebServer::create_web_server();
        let sys_loop_clone = sys_loop.clone();
        let mut sub = Mutex::new(BTreeMap::<i32, EspSubscription<System>>::new());

        Self::register_static_resource(&mut server, "/", INDEX_HTML, "text/html");
        Self::register_static_resource(&mut server, "/stylesheet.css", INDEX_CSS, "text/css");
        Self::register_static_resource(&mut server, "/index.js", INDEX_JS, "text/javascript");

        server
            .ws_handler("/ws/tracker", move |ws| {
                if ws.is_closed() {
                    let mut subscriptions = sub.lock().unwrap();
                    let session_id = ws.session();
                    if subscriptions.contains_key(&session_id) {
                        subscriptions.remove(&session_id);
                    }
                    return Ok(());
                }

                if ws.is_new() {
                    Self::register_subscriptions(&*ws, &sub, &sys_loop_clone);
                    return Ok(());
                }

                let (_frame_type, len) = ws.recv(&mut []).unwrap();
                if len != COMMAND_LEN {
                    return Err(EspError::from_infallible::<ESP_ERR_INVALID_SIZE>());
                }
                let mut buf = [0; COMMAND_LEN];
                ws.recv(buf.as_mut())?;

                let command: u16 = ((buf[0] as u16) << 8) + buf[1] as u16;
                web::protocol::map_command(&handler, command);

                return Ok::<(), EspError>(());
            })
            .unwrap();

        WebServer {
            http_server: server,
            sys_loop,
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

    fn register_subscriptions(
        ws: &EspHttpWsConnection,
        subscriptions: &Mutex<BTreeMap<i32, EspSubscription<System>>>,
        sys_loop: &EspEventLoop<System>,
    ) {
        let mut subscriptions = subscriptions.lock().unwrap();
        if subscriptions.contains_key(&ws.session()) {
            return;
        }

        let mut detached_sender = ws.create_detached_sender().unwrap();
        let subscription = sys_loop
            .subscribe::<StepperEvent, _>(move |event| {
                detached_sender
                    .send(
                        FrameType::Binary(false),
                        "Please enter a number between 1 and 100".as_bytes(),
                    )
                    .unwrap();
            })
            .unwrap();
        subscriptions.insert(ws.session(), subscription);

        let mut detached_sender = ws.create_detached_sender().unwrap();
        let subscription = sys_loop
            .subscribe::<SystemEvent, _>(move |event| {
                detached_sender
                    .send(
                        FrameType::Binary(false),
                        "Please enter a number between 1 and 100".as_bytes(),
                    )
                    .unwrap();
            })
            .unwrap();
        subscriptions.insert(ws.session(), subscription);
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
