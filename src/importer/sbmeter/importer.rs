extern crate alloc;

use crate::importer::sbmeter::bluetooth::run;
use bt_hci::cmd::le::{LeSetScanEnable, LeSetScanParams};
use bt_hci::controller::ControllerCmdSync;
use trouble_host::prelude::*;

pub struct Importer<C>
where
    C: Controller + ControllerCmdSync<LeSetScanParams> + ControllerCmdSync<LeSetScanEnable>,
{
    controller: C,
}

impl<C> Importer<C>
where
    C: Controller + ControllerCmdSync<LeSetScanParams> + ControllerCmdSync<LeSetScanEnable>,
{
    pub fn new(controller: C) -> Self {
        Importer { controller }
    }

    pub async fn run(self) {
        run(self.controller).await;
    }
}
