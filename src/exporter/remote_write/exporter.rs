extern crate alloc;

use crate::{
    networking::NetworkingStack,
    repository::{Config, MetricsRepository},
};
use alloc::rc::Rc;
use core::{cell::RefCell, future::pending};

pub struct Exporter {
    networking_stack: Rc<RefCell<NetworkingStack>>,
    repository: Rc<RefCell<MetricsRepository>>,
}

impl Exporter {
    pub fn new(
        networking_stack: Rc<RefCell<NetworkingStack>>,
        repository: Rc<RefCell<MetricsRepository>>,
        _config: &Config,
    ) -> Self {
        Self {
            networking_stack,
            repository,
        }
    }

    pub async fn run(self) {
        let (_, _) = (self.networking_stack, self.repository);
        pending::<()>().await;
    }
}
