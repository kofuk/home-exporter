extern crate alloc;

use crate::{
    networking::{NetworkingStack, TcpClientState},
    repository::{Config, MetricsRepository},
    time::Time,
};
use alloc::rc::Rc;
use core::{future::pending, net::SocketAddrV4};
use defmt::*;
use embassy_net::IpAddress;
use embassy_rp::clocks::RoscRng;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::Duration;
use embedded_nal_async::TcpConnect;
use reqwless::{
    client::{HttpClient, TlsConfig},
    request::RequestBuilder,
};

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

    async fn real_run(self) -> Result<(), ()> {
        let net_stack = self.networking_stack.lock().await;

        let addr = match net_stack.net.query_dns_a("example.com").await {
            Ok(IpAddress::Ipv4(addr)) => addr,
            _ => return Err(()),
        };

        let tcp_client_state = TcpClientState::default();
        let mut socket = net_stack.net.new_tcp_client(&tcp_client_state);
        socket.set_timeout(Some(Duration::from_secs(10)));
        match socket
            .connect(core::net::SocketAddr::V4(SocketAddrV4::new(addr, 443)))
            .await
        {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to connect: {:?}", e);
                return Err(());
            }
        }

        let dns = net_stack.net.get_dns_client();

        let mut read_record_buffer = [0u8; 16384];
        let mut write_record_buffer = [0u8; 16384];

        let mut client = HttpClient::new_with_tls(
            &socket,
            &dns,
            TlsConfig::new(
                RoscRng {}.next_u64(),
                &mut read_record_buffer,
                &mut write_record_buffer,
                reqwless::client::TlsVerify::None,
            ),
        );

        let mut rx_buffer = [0u8; 1024];
        let mut request = client
            .request(reqwless::request::Method::GET, "https://example.com")
            .await
            .map_err(|err| {
                error!("Failed to create request: {:?}", err);
                ()
            })?
            .headers(&[("Host", "example.com")]);
        let response = request.send(&mut rx_buffer).await.map_err(|err| {
            error!("Failed to send request: {:?}", err);
            ()
        })?;

        info!("Received response with status: {:?}", response.status);

        let (_, _) = (self.repository, self.time);

        Ok(())
    }

    pub async fn run(self) {
        let _ = self.real_run().await;

        pending::<()>().await;
    }
}
