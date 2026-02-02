extern crate alloc;

use core::cell::RefCell;

use crate::importer::sbmeter::adv::ScanHandler;
use crate::importer::sbmeter::bluetooth::run;
use crate::repository::{Config, MetricsRepository};
use alloc::rc::Rc;
use bt_hci::cmd::le::{LeSetScanEnable, LeSetScanParams};
use bt_hci::controller::ControllerCmdSync;
use trouble_host::prelude::*;

pub struct Importer<C>
where
    C: Controller + ControllerCmdSync<LeSetScanParams> + ControllerCmdSync<LeSetScanEnable>,
{
    controller: C,
    scan_handler: ScanHandler,
}

impl<C> Importer<C>
where
    C: Controller + ControllerCmdSync<LeSetScanParams> + ControllerCmdSync<LeSetScanEnable>,
{
    pub fn new(controller: C, repository: Rc<RefCell<MetricsRepository>>, config: &Config) -> Self {
        let mut scan_handler = ScanHandler::new(repository);
        if let Some(mac_addr) = &config.importer.sbmeter.mac_addr {
            match scan_handler.set_target_mac_addr(&mac_addr) {
                Err(e) => defmt::error!("Error: {}", e),
                _ => (),
            }
        }

        Importer {
            controller,
            scan_handler,
        }
    }

    pub async fn run(self) {
        run(self.controller, self.scan_handler).await;
    }
}
