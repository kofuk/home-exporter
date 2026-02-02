use crate::networking::bluetooth::Bluetooth;
use cyw43::bluetooth::BtDriver;
use embassy_executor::Spawner;
use trouble_host::prelude::ExternalController;
use {defmt_rtt as _, panic_probe as _};

pub struct NetworkingStack {
    #[cfg(feature = "bluetooth")]
    pub bluetooth: Bluetooth,
}

impl NetworkingStack {
    pub async fn new(
        spawner: Spawner,
        controller: ExternalController<BtDriver<'static>, 10>,
    ) -> Self {
        NetworkingStack {
            #[cfg(feature = "bluetooth")]
            bluetooth: Bluetooth::new(controller, spawner),
        }
    }
}
