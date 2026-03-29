#[cfg(feature = "bluetooth")]
use crate::networking::bluetooth::Bluetooth;
#[cfg(feature = "wifi")]
use crate::networking::net::Net;
#[cfg(feature = "bluetooth")]
use cyw43::bluetooth::BtDriver;
#[cfg(feature = "bluetooth")]
use embassy_executor::Spawner;
use embassy_net::Stack;
#[cfg(feature = "bluetooth")]
use trouble_host::prelude::ExternalController;
use {defmt_rtt as _, panic_probe as _};

pub struct NetworkingStack {
    #[cfg(feature = "bluetooth")]
    pub bluetooth: Bluetooth,
    #[cfg(feature = "wifi")]
    pub net: Net,
}

impl NetworkingStack {
    pub async fn new(
        #[cfg(feature = "bluetooth")] spawner: Spawner,
        #[cfg(feature = "bluetooth")] controller: ExternalController<BtDriver<'static>, 10>,
        #[cfg(feature = "wifi")] net_stack: Stack<'static>,
    ) -> Self {
        #[cfg(feature = "wifi")]
        NetworkingStack {
            #[cfg(feature = "bluetooth")]
            bluetooth: Bluetooth::new(controller, spawner),
            #[cfg(feature = "wifi")]
            net: Net::new(net_stack),
        }
    }
}
