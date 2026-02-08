extern crate alloc;

use crate::{
    networking::NetworkingStack,
    repository::{Config, MetricsRepository},
    time::Time,
};
use alloc::rc::Rc;
use core::future::pending;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};

pub struct Exporter {
    networking_stack: Rc<Mutex<NoopRawMutex, NetworkingStack>>,
    repository: Rc<Mutex<NoopRawMutex, MetricsRepository>>,
    time: Rc<Mutex<NoopRawMutex, Time>>,
}

impl Exporter {
    pub fn new(
        networking_stack: Rc<Mutex<NoopRawMutex, NetworkingStack>>,
        repository: Rc<Mutex<NoopRawMutex, MetricsRepository>>,
        time: Rc<Mutex<NoopRawMutex, Time>>,
        _config: &Config,
    ) -> Self {
        Self {
            networking_stack,
            repository,
            time,
        }
    }

    pub async fn run(self) {
        let (_, _, _) = (self.networking_stack, self.repository, self.time);
        pending::<()>().await;
    }
}
