use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{esp_wifi_set_max_tx_power, EspError};
use esp_idf_svc::wifi;
use esp_idf_svc::wifi::{AccessPointConfiguration, AuthMethod, BlockingWifi, EspWifi};

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

const CHANNEL: u8 = 11;
const MAX_CONNECTIONS: u16 = 1;

pub struct WifiConnection {
    wifi: BlockingWifi<EspWifi<'static>>
}

impl WifiConnection {
    pub fn new(modem: Modem, sys_loop: EspSystemEventLoop, nvs: Option<EspDefaultNvsPartition>) -> Self {
        let mut wifi = BlockingWifi::wrap(
            EspWifi::new(modem, sys_loop.clone(), nvs).unwrap(),
            sys_loop,
        ).unwrap();

        let wifi_config = wifi::Configuration::AccessPoint(AccessPointConfiguration {
            ssid: SSID.try_into().unwrap(),
            ssid_hidden: false,
            channel: CHANNEL,
            auth_method: AuthMethod::WPA2Personal,
            password: PASSWORD.try_into().unwrap(),
            max_connections: MAX_CONNECTIONS,
            ..Default::default()
        });

        wifi.set_configuration(&wifi_config).unwrap();

        WifiConnection {
            wifi
        }
    }

    pub fn connect(&mut self) -> Result<(), EspError> {
        self.wifi.start()?;
        self.wifi.wait_netif_up()?;

        // ESP super mini antenna design is broken and only works with reduced tx power
        // see https://forum.arduino.cc/t/no-wifi-connect-with-esp32-c3-super-mini/1324046/12
        WifiConnection::set_wifi_tx_power(12.0)?;

        Ok::<(), EspError>(())
    }

    fn set_wifi_tx_power(dbm: f32) -> Result<(), EspError> {
        let power = (dbm * 4.0) as i8;
        let res = unsafe { esp_wifi_set_max_tx_power(power) };
        if res == 0 {
            Ok(())
        } else {
            Err(EspError::from(res).unwrap())
        }
    }
}
