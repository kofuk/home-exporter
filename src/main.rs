#![no_std]
#![no_main]

extern crate alloc;

use alloc::rc::Rc;
use core::cell::RefCell;
use core::future::pending;
use core::ptr::addr_of_mut;
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
#[cfg(feature = "sbmeter")]
use home_exporter::importer::sbmeter::Importer;
use home_exporter::repository::MetricsRepository;
use home_exporter::repository::Config;
use linked_list_allocator::LockedHeap;
use static_cell::StaticCell;
use trouble_host::prelude::ExternalController;
use {defmt_rtt as _, panic_probe as _};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE: usize = 1024 * 8;
static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

fn init_global_allocator() {
    unsafe {
        let heap_start = addr_of_mut!(HEAP_MEM) as *mut u8;
        ALLOCATOR.lock().init(heap_start, HEAP_SIZE);
    }
}

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[cfg(feature = "sbmeter")]
#[embassy_executor::task]
async fn sbmeter_importer_task(importer: Importer<ExternalController<BtDriver<'static>, 10>>) {
    importer.run().await
}

#[cfg(any(feature = "bluetooth", feature = "wifi"))]
async fn init_net_device(
    spawner: Spawner,
) -> (Device<'static>, Option<BtDriver<'static>>, Control<'static>) {
    let p = embassy_rp::init(Default::default());

    let fw = include_bytes!("../firmware/43439A0.bin");
    let clm = include_bytes!("../firmware/43439A0_clm.bin");
    #[cfg(feature = "bluetooth")]
    let btfw = include_bytes!("../firmware/43439A0_btfw.bin");

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
    #[cfg(feature = "wifi")]
    let (net_device, bt_device, mut control, runner) = {
        let (net_device, control, runner) = cyw43::new(state, pwr, spi, fw).await;
        (net_device, None, control, runner)
    };

    unwrap!(spawner.spawn(cyw43_task(runner)));
    control.init(clm).await;

    return (net_device, bt_device, control);
}

fn load_config() -> Result<Config, serde_json_core::de::Error> {
    let config_data = include_bytes!("../config.json");
    match serde_json_core::from_slice(config_data) {
        Ok((config, _)) => Ok(config),
        Err(e) => {
            Err(e)
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_global_allocator();

    let config = load_config().unwrap();

    let repository = Rc::from(RefCell::from(MetricsRepository::new()));

    #[cfg(any(feature = "bluetooth", feature = "wifi"))]
    let (_net_device, bt_device, _control) = init_net_device(spawner).await;

    #[cfg(feature = "sbmeter")]
    {
        let controller: ExternalController<_, 10> = ExternalController::new(bt_device.unwrap());
        unwrap!(spawner.spawn(sbmeter_importer_task(Importer::new(
            controller,
            repository.clone(),
            &config
        ))));
    }

    pending::<()>().await;
}
