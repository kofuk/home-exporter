use cyw43::bluetooth::BtDriver;
use defmt::*;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Duration;
use static_cell::StaticCell;
use trouble_host::prelude::*;

/// Max number of connections
const CONNECTIONS_MAX: usize = 1;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 3; // Signal + att + CoC

const SCAN_QUEUE_SIZE: usize = 5;

pub struct ScanResult {
    pub addr: BdAddr,
    pub data: heapless::Vec<u8, 31>, // 広告データ
    pub rssi: i8,
}

struct ScanHandler {
    channel: &'static Channel<CriticalSectionRawMutex, ScanResult, SCAN_QUEUE_SIZE>,
}

impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, reports: bt_hci::param::LeAdvReportsIter) {
        for report in reports {
            match report {
                Ok(report) => {
                    let mut data = heapless::Vec::<u8, 31>::new();
                    if let Err(_) = data.extend_from_slice(report.data) {
                        warn!("Advertisement data too long, dropping extra bytes");
                    } else {
                        match self.channel.try_send(ScanResult {
                            addr: report.addr,
                            data,
                            rssi: report.rssi,
                        }) {
                            Ok(()) => (),
                            Err(_) => {
                                warn!("Scan result channel full, dropping advertisement");
                            }
                        }
                    }
                }
                Err(e) => {
                    info!("Error processing advertisement report: {:?}", e);
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn bluetooth_task(
    mut runner: Runner<'static, ExternalController<BtDriver<'static>, 10>, DefaultPacketPool>,
    handler: ScanHandler,
) -> ! {
    loop {
        match runner.run_with_handler(&handler).await {
            Err(_) => {
                warn!("Bluetooth runner error, restarting...");
            }
            _ => (),
        }

        // just in case, wait a bit before restarting
        embassy_time::Timer::after_secs(3).await;
    }
}

pub struct Bluetooth {
    scanner: Scanner<'static, ExternalController<BtDriver<'static>, 10>, DefaultPacketPool>,
}

impl Bluetooth {
    pub fn new(controller: ExternalController<BtDriver<'static>, 10>, spawner: Spawner) -> Self {
        static RESOURCES: StaticCell<
            HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>,
        > = StaticCell::new();
        static STACK: StaticCell<
            Stack<'static, ExternalController<BtDriver<'static>, 10>, DefaultPacketPool>,
        > = StaticCell::new();
        static SCAN_CHANNEL: StaticCell<
            Channel<CriticalSectionRawMutex, ScanResult, SCAN_QUEUE_SIZE>,
        > = StaticCell::new();

        let resources: &'static mut HostResources<_, _, _> = RESOURCES.init(HostResources::new());
        let scan_channel = SCAN_CHANNEL.init(Channel::new());
        let stack = STACK.init(trouble_host::new(controller, resources));
        let Host {
            central, runner, ..
        } = stack.build();

        spawner.must_spawn(bluetooth_task(
            runner,
            ScanHandler {
                channel: scan_channel,
            },
        ));

        Bluetooth {
            scanner: Scanner::new(central),
        }
    }

    pub async fn start_scan(&mut self) {
        let config = ScanConfig {
            active: false,
            interval: Duration::from_secs(3),
            window: Duration::from_secs(3),
            ..Default::default()
        };

        loop {
            let session = self.scanner.scan(&config).await.unwrap();
            embassy_time::Timer::after_secs(3).await;
            drop(session);
            embassy_time::Timer::after_secs(3).await;
        }
    }
}
