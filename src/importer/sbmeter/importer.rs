extern crate alloc;

use crate::importer::sbmeter::adv::ScanHandler;
use crate::networking::NetworkingStack;
use crate::networking::bluetooth::ScanResult;
use crate::repository::{Config, MetricsRepository};
use alloc::rc::Rc;
use core::cell::RefCell;

pub struct Importer {
    networking_stack: Rc<RefCell<NetworkingStack>>,
    scan_handler: ScanHandler,
}

impl Importer {
    pub fn new(
        networking_stack: Rc<RefCell<NetworkingStack>>,
        repository: Rc<RefCell<MetricsRepository>>,
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
            .borrow_mut()
            .bluetooth
            .start_scan(|scan_result: ScanResult| {
                self.scan_handler.process_report(
                    scan_result.addr,
                    scan_result.rssi,
                    &scan_result.data,
                );
            })
            .await;
    }
}
