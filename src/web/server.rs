use std::collections::BTreeMap;
use std::sync::Mutex;
use crate::stepper::{StepperDirection, StepperEvent};
use crate::web;
use crate::web::protocol::CallbackHandler;
use esp_idf_svc::eventloop::{EspEvent, EspEventLoop, EspSubscription, System};
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::sys::{EspError, ESP_ERR_INVALID_SIZE};
use esp_idf_svc::ws::FrameType;
use log::info;
use std::thread;

static INDEX_HTML: &str = include_str!("webapp/index.html");
static INDEX_CSS: &str = include_str!("webapp/stylesheet.css");

static INDEX_JS: &str = include_str!("webapp/index.js");

const COMMAND_LEN: usize = 2;

pub struct WebServer {
    http_server: EspHttpServer<'static>,
    sys_loop: EspEventLoop<System>,
    // event_subscription: Option<EspSubscription<'static, System>>
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

        server
            .fn_handler("/", Method::Get, |req| {
                req.into_ok_response()?
                    .write_all(INDEX_HTML.as_bytes())
                    .map(|_| ())
            })
            .unwrap();

        // TODO: Improve, do not repeat!
        server
            .fn_handler("/stylesheet.css", Method::Get, |req| {
                let headers = [("Content-Type", "text/css")];
                req.into_response(200, Some("OK"), &headers)?
                    .write_all(INDEX_CSS.as_bytes())
                    .map(|_| ())
            })
            .unwrap();

        server
            .fn_handler("/index.js", Method::Get, |req| {
                let headers = [("Content-Type", "text/javascript")];
                req.into_response(200, Some("OK"), &headers)?
                    .write_all(INDEX_JS.as_bytes())
                    .map(|_| ())
            })
            .unwrap();

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
                    let mut subscriptions = sub.lock().unwrap();
                    if subscriptions.contains_key(&ws.session()) {
                        return Ok(())
                    }

                    let mut detached_sender = ws.create_detached_sender().unwrap();

                    let subscription = sys_loop_clone.subscribe::<StepperEvent, _>(move |event| {
                        info!("[Subscribe callback] Got event: {event:?}");
                        detached_sender
                            .send(
                                FrameType::Text(false),
                                "Please enter a number between 1 and 100".as_bytes(),
                            )
                            .unwrap();
                    })?;
                    subscriptions.insert(ws.session(), subscription);

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
