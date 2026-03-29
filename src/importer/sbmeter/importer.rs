extern crate alloc;

use crate::importer::sbmeter::adv::ScanHandler;
use crate::networking::NetworkingStack;
use crate::networking::bluetooth::ScanResult;
use crate::repository::{Config, MetricsRepository};
use alloc::rc::Rc;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;

pub struct Importer {
    networking_stack: Rc<Mutex<NoopRawMutex, NetworkingStack>>,
    scan_handler: ScanHandler,
}

impl Importer {
    pub fn new(
        networking_stack: Rc<Mutex<NoopRawMutex, NetworkingStack>>,
        repository: Rc<Mutex<NoopRawMutex, MetricsRepository>>,
        config: &Config,
    ) -> Self {
        let mut scan_handler = ScanHandler::new(repository);
        if let Some(mac_addr) = &config.importer.sbmeter.mac_addr {
            match scan_handler.set_target_mac_addr(&mac_addr) {
                Err(e) => defmt::error!("Error: {}", e),
                _ => (),
            }
        }

        Importer {
            networking_stack,
            scan_handler,
        }
    }

    pub async fn run(self) {
        self.networking_stack
            .lock()
            .await
            .bluetooth
            .start_scan(async |scan_result: ScanResult| {
                self.scan_handler
                    .process_report(scan_result.addr, scan_result.rssi, &scan_result.data)
                    .await;
            })
            .await;
    }
}
