use crate::networking::bluetooth::Bluetooth;
#[cfg(feature = "wifi")]
use crate::networking::net::Net;
use cyw43::bluetooth::BtDriver;
use embassy_executor::Spawner;
use embassy_net::Stack;
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
        spawner: Spawner,
        #[cfg(feature = "bluetooth")] controller: ExternalController<BtDriver<'static>, 10>,
        #[cfg(feature = "wifi")] net_stack: Stack<'static>,
    ) -> Self {
        NetworkingStack {
            #[cfg(feature = "bluetooth")]
            bluetooth: Bluetooth::new(controller, spawner),
            #[cfg(feature = "wifi")]
            net: Net::new(net_stack),
        }
    }
}
