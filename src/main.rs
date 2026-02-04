#![no_std]
#![no_main]

extern crate alloc;

use alloc::rc::Rc;
use core::cell::RefCell;
use core::future::pending;
use core::ptr::addr_of_mut;
use cyw43::JoinOptions;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Config as NetConfig, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
#[cfg(feature = "sbmeter")]
use home_exporter::importer::sbmeter::Importer;
use home_exporter::networking::NetworkingStack;
use home_exporter::repository::Config;
use home_exporter::repository::MetricsRepository;
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

#[cfg(feature = "sbmeter")]
#[embassy_executor::task]
async fn sbmeter_importer_task(importer: Importer) {
    importer.run().await
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

fn load_config() -> Result<Config, serde_json_core::de::Error> {
    let config_data = include_bytes!("../config.json");
    match serde_json_core::from_slice(config_data) {
        Ok((config, _)) => Ok(config),
        Err(e) => Err(e),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_global_allocator();

    let config = load_config().unwrap();
    let repository = Rc::from(RefCell::from(MetricsRepository::new()));

    let mut rng = RoscRng;

    // DO NOT refactor the code below into separate functions or scopes - doing so will trigger internal bugs

    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    let p = embassy_rp::init(Default::default());

    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    let fw = include_bytes!("../firmware/43439A0.bin");
    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    let clm = include_bytes!("../firmware/43439A0_clm.bin");
    #[cfg(feature = "bluetooth")]
    let btfw = include_bytes!("../firmware/43439A0_btfw.bin");

    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    let pwr = Output::new(p.PIN_23, Level::Low);
    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    let cs = Output::new(p.PIN_25, Level::High);
    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    let mut pio = Pio::new(p.PIO0, Irqs);
    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
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

    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
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

    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    unwrap!(spawner.spawn(cyw43_task(runner)));
    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    control.init(clm).await;
    #[cfg(any(feature = "wifi", feature = "bluetooth"))]
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    #[cfg(feature = "bluetooth")]
    let controller: ExternalController<_, 10> = ExternalController::new(bt_device.unwrap());

    #[cfg(feature = "wifi")]
    let net_config = NetConfig::dhcpv4(Default::default());
    #[cfg(feature = "wifi")]
    let seed = rng.next_u64();

    #[cfg(feature = "wifi")]
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    #[cfg(feature = "wifi")]
    let (net_stack, runner) = embassy_net::new(
        net_device,
        net_config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    #[cfg(feature = "wifi")]
    unwrap!(spawner.spawn(net_task(runner)));

    #[cfg(feature = "wifi")]
    while let Err(_) = control
        .join(
            &config.wifi.ssid,
            JoinOptions::new(config.wifi.password.as_bytes()),
        )
        .await
    {
        info!("Failed to join WiFi network");
    }

    #[cfg(feature = "wifi")]
    info!("Waiing for network link up...");
    #[cfg(feature = "wifi")]
    net_stack.wait_link_up().await;

    #[cfg(feature = "wifi")]
    info!("Waiting for DHCP...");
    #[cfg(feature = "wifi")]
    net_stack.wait_config_up().await;

    #[cfg(feature = "wifi")]
    info!("Network ready!");

    #[cfg(all(feature = "bluetooth", feature = "wifi"))]
    let stack = Rc::from(RefCell::from(
        NetworkingStack::new(spawner, controller, net_stack).await,
    ));
    #[cfg(not(feature = "wifi"))]
    let stack = Rc::from(RefCell::from(
        NetworkingStack::new(spawner, controller).await,
    ));
    #[cfg(not(feature = "bluetooth"))]
    let stack = Rc::from(RefCell::from(
        NetworkingStack::new(spawner, net_stack).await,
    ));

    // End of no-refactor zone

    #[cfg(feature = "sbmeter")]
    {
        unwrap!(spawner.spawn(sbmeter_importer_task(Importer::new(
            stack.clone(),
            repository.clone(),
            &config
        ))));
    }

    pending::<()>().await;
}
