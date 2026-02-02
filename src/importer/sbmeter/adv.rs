extern crate alloc;

use crate::repository::MetricsRepository;
use alloc::rc::Rc;
use core::cell::RefCell;
use defmt::info;
use heapless::{String, Vec};
use trouble_host::prelude::*;

#[derive(Debug)]
pub struct ThermoHygroData {
    pub temperature: f32,
    pub humidity: u8,
}

fn parse_thermo_hygro_data(data: &[u8]) -> Option<ThermoHygroData> {
    if data.len() < 11 {
        return None;
    }

    let temp_decimal_byte = data[8];
    let temp_integer_byte = data[9];
    let humidity_byte = data[10];

    // Temperature
    let temperature_decimal = (temp_decimal_byte & 0x0F) as f32;
    let temperature_positive = (temp_integer_byte & 0x80) != 0;
    let temperature_integer = (temp_integer_byte & 0x7F) as f32;

    let mut temperature = temperature_integer + temperature_decimal / 10.0;
    if !temperature_positive {
        temperature = -temperature;
    }

    // Humidity
    let humidity = humidity_byte & 0x7F;

    Some(ThermoHygroData {
        temperature,
        humidity,
    })
}

fn extract_manufacturer_data(adv_data: &[u8]) -> Option<(u16, &[u8])> {
    let mut i = 0;
    while i < adv_data.len() {
        let len = adv_data[i] as usize;
        if len == 0 || i + len >= adv_data.len() {
            break;
        }

        let ad_type = adv_data[i + 1];
        // 0xFF = Manufacturer Specific Data
        if ad_type == 0xFF {
            let data_start = i + 2;
            let data_end = i + 1 + len;

            if data_end <= adv_data.len() && data_end - data_start >= 2 {
                // Company ID は Little Endian
                let company_id =
                    u16::from_le_bytes([adv_data[data_start], adv_data[data_start + 1]]);
                let manufacturer_data = &adv_data[data_start + 2..data_end];
                return Some((company_id, manufacturer_data));
            }
        }

        i += len + 1;
    }
    None
}

fn extract_local_name(adv_data: &[u8]) -> Option<&str> {
    let mut i = 0;
    while i < adv_data.len() {
        let len = adv_data[i] as usize;
        if len == 0 || i + len >= adv_data.len() {
            break;
        }

        let ad_type = adv_data[i + 1];
        // 0x09 = Complete Local Name, 0x08 = Shortened Local Name
        if ad_type == 0x09 || ad_type == 0x08 {
            return core::str::from_utf8(&adv_data[i + 2..i + 1 + len]).ok();
        }

        i += len + 1;
    }
    None
}

pub struct ScanHandler {
    repository: Rc<RefCell<MetricsRepository>>,
    filter_addr: Option<BdAddr>,
}

impl ScanHandler {
    pub fn new(repository: Rc<RefCell<MetricsRepository>>) -> Self {
        Self {
            repository,
            filter_addr: None,
        }
    }

    pub fn set_target_mac_addr(&mut self, addr: &String<17>) -> Result<(), &'static str> {
        let parts: Vec<&str, 6> = addr.split(':').collect();
        if parts.len() != 6 {
            return Err("Error parsing mac address");
        }
        let parts: Vec<u8, 6> = parts
            .iter()
            .map(|e| u8::from_str_radix(e, 16).unwrap_or(0))
            .collect();
        self.filter_addr = Some(BdAddr::new([
            parts[5], parts[4], parts[3], parts[2], parts[1], parts[0],
        ]));

        Ok(())
    }

    fn process_report(&self, addr: BdAddr, rssi: i8, data: &[u8]) {
        let local_name = extract_local_name(data);

        let manufacturer_data = match extract_manufacturer_data(data) {
            Some(data) => data,
            None => return,
        };

        let (company_id, mfg_data) = manufacturer_data;

        // Mac Address (littele endian)
        if let Some(target_addr) = &self.filter_addr {
            if addr != *target_addr {
                return;
            }
        } else {
            let addr = addr.raw();
            info!(
                "device detected: addr={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}, rssi={}, localname={:?}, companyid={:04x}",
                addr[5], addr[4], addr[3], addr[2], addr[1], addr[0], rssi, local_name, company_id
            );
        }

        match parse_thermo_hygro_data(mfg_data) {
            Some(data) => {
                info!(
                    "Temperature: {}°C, Humidity: {}%",
                    data.temperature, data.humidity
                );
                let mut repository = self.repository.borrow_mut();
                repository.set_temperature(data.temperature);
                repository.set_humidity(data.humidity);
            }
            None => {
                info!("insufficient data length for thermo/hygro");
            }
        }
    }
}

impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, reports: bt_hci::param::LeAdvReportsIter) {
        for report in reports {
            match report {
                Ok(report) => {
                    self.process_report(report.addr, report.rssi, report.data);
                }
                Err(e) => {
                    info!("Error processing advertisement report: {:?}", e);
                }
            }
        }
    }
}
