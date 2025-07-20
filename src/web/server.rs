use crate::stepper::StepperDirection;
use crate::web;
use crate::web::protocol::CallbackHandler;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::sys::{EspError, ESP_ERR_INVALID_SIZE};

static INDEX_HTML: &str = include_str!("webapp/index.html");
static INDEX_CSS: &str = include_str!("webapp/stylesheet.css");

static INDEX_JS: &str = include_str!("webapp/index.js");

const COMMAND_LEN: usize = 2;

pub struct WebServer {
    http_server: EspHttpServer<'static>,
}

impl WebServer {
    pub fn new<M, T>(handler: CallbackHandler<M, T>) -> Self
    where
        M: Fn(StepperDirection, u16) + Send + Sync + 'static,
        T: Fn(bool) + Send + Sync + 'static,
    {
        let mut server = WebServer::create_web_server();

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
                if ws.is_new() || ws.is_closed() {
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
            &[]
        ).unwrap();
        core::mem::forget(mdns);

        EspHttpServer::new(&server_configuration).unwrap()
    }
}
