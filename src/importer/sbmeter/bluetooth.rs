extern crate alloc;

use crate::importer::sbmeter::adv::ScanHandler;
use bt_hci::cmd::le::{LeSetScanEnable, LeSetScanParams};
use bt_hci::controller::ControllerCmdSync;
use embassy_futures::join::join;
use trouble_host::prelude::*;

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

pub async fn run<C>(controller: C)
where
    C: Controller + ControllerCmdSync<LeSetScanParams> + ControllerCmdSync<LeSetScanEnable>,
{
    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();

    let stack = trouble_host::new(controller, &mut resources);
    let Host {
        central,
        mut runner,
        ..
    } = stack.build();

    let config = ScanConfig {
        active: false,
        ..Default::default()
    };

    let mut scanner = Scanner::new(central);

    let handler = ScanHandler;

    let _ = join(runner.run_with_handler(&handler), async {
        loop {
            let session = scanner.scan(&config).await.unwrap();
            embassy_time::Timer::after_secs(3).await;
            drop(session);
            embassy_time::Timer::after_secs(3).await;
        }
    })
    .await;
}
