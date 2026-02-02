extern crate alloc;

use core::cell::RefCell;

use crate::importer::sbmeter::bluetooth::run;
use crate::repository::MetricsRepository;
use alloc::rc::Rc;
use bt_hci::cmd::le::{LeSetScanEnable, LeSetScanParams};
use bt_hci::controller::ControllerCmdSync;
use trouble_host::prelude::*;

pub struct Importer<C>
where
    C: Controller + ControllerCmdSync<LeSetScanParams> + ControllerCmdSync<LeSetScanEnable>,
{
    controller: C,
    repository: Rc<RefCell<MetricsRepository>>,
}

impl<C> Importer<C>
where
    C: Controller + ControllerCmdSync<LeSetScanParams> + ControllerCmdSync<LeSetScanEnable>,
{
    pub fn new(controller: C, repository: Rc<RefCell<MetricsRepository>>) -> Self {
        Importer {
            controller,
            repository,
        }
    }

    pub async fn run(self) {
        run(self.controller, self.repository).await;
    }
}
