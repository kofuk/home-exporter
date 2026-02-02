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

#[derive(Deserialize)]
pub struct ExporterConfig {
    pub location: String<32>,
}

#[cfg(feature = "wifi")]
#[derive(Deserialize)]
pub struct WiFiConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub importer: ImporterConfig,
    #[cfg(feature = "wifi")]
    pub wifi: WiFiConfig,
    pub exporter: ExporterConfig,
}
