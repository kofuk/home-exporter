use crate::networking::bluetooth::Bluetooth;
use cyw43::Control;
use cyw43::bluetooth::BtDriver;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net_wiznet::Device;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;
use trouble_host::prelude::ExternalController;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

async fn init_net_device(
    spawner: Spawner,
) -> (Device<'static>, Option<BtDriver<'static>>, Control<'static>) {
    let p = embassy_rp::init(Default::default());

    let fw = include_bytes!("../../firmware/43439A0.bin");
    let clm = include_bytes!("../../firmware/43439A0_clm.bin");
    #[cfg(feature = "bluetooth")]
    let btfw = include_bytes!("../../firmware/43439A0_btfw.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());

    #[cfg(feature = "bluetooth")]
    let (net_device, bt_device, mut control, runner) = {
        let (net_device, bt_device, control, runner) =
            cyw43::new_with_bluetooth(state, pwr, spi, fw, btfw).await;
        (net_device, Some(bt_device), control, runner)
    };
    #[cfg(not(feature = "bluetooth"))]
    let (net_device, bt_device, mut control, runner) = {
        let (net_device, control, runner) = cyw43::new(state, pwr, spi, fw).await;
        (net_device, None, control, runner)
    };

    unwrap!(spawner.spawn(cyw43_task(runner)));
    control.init(clm).await;

    return (net_device, bt_device, control);
}

pub struct Stack {
    #[cfg(feature = "bluetooth")]
    pub bluetooth: Bluetooth,
}

impl Stack {
    pub async fn new(spawner: Spawner) -> Self {
        let (_net_device, bt_device, _control) = init_net_device(spawner).await;

        let controller: ExternalController<_, 10> = ExternalController::new(bt_device.unwrap());

        Stack {
            #[cfg(feature = "bluetooth")]
            bluetooth: Bluetooth::new(controller, spawner),
        }
    }
}
