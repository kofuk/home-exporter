extern crate alloc;

use heapless::String;
use serde::Deserialize;

#[cfg(feature = "sbmeter")]
#[derive(Deserialize)]
pub struct SbMeterConfig {
    pub mac_addr: Option<String<17>>,
}

#[derive(Deserialize)]
pub struct ImporterConfig {
    #[cfg(feature = "sbmeter")]
    pub sbmeter: SbMeterConfig,
}

#[cfg(feature = "remote-write")]
#[derive(Deserialize)]
pub struct RemoteWriteConfig {
    pub endpoint: String<64>,
    pub username: String<64>,
    pub password: String<64>,
}

#[derive(Deserialize)]
pub struct ExporterConfig {
    pub location_label: String<32>,
    #[cfg(feature = "remote-write")]
    pub remote_write: RemoteWriteConfig,
}

#[cfg(feature = "wifi")]
#[derive(Deserialize)]
pub struct WiFiConfig {
    pub ssid: String<16>,
    pub password: String<32>,
}

#[derive(Deserialize)]
pub struct Config {
    pub importer: ImporterConfig,
    pub exporter: ExporterConfig,
    #[cfg(feature = "wifi")]
    pub wifi: WiFiConfig,
}
