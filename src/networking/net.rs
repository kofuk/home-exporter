use defmt::*;
use embassy_net::tcp::client as etc;
use embassy_net::udp::{self as eu, PacketMetadata};
use embassy_net::{IpAddress, Stack};

pub struct Net {
    net_stack: Stack<'static>,
}

pub struct TcpClientState {
    state: etc::TcpClientState<1, 4096, 4096>,
}

impl Default for TcpClientState {
    fn default() -> Self {
        TcpClientState {
            state: etc::TcpClientState::new(),
        }
    }
}

pub struct UdpSocketData {
    tx_meta: [PacketMetadata; 1],
    tx_buffer: [u8; 512],
    rx_meta: [PacketMetadata; 1],
    rx_buffer: [u8; 512],
}

impl Default for UdpSocketData {
    fn default() -> Self {
        Self {
            tx_meta: [PacketMetadata::EMPTY; 1],
            tx_buffer: [0; 512],
            rx_meta: [PacketMetadata::EMPTY; 1],
            rx_buffer: [0; 512],
        }
    }
}

impl Net {
    pub fn new(net_stack: Stack<'static>) -> Self {
        Net { net_stack }
    }

    pub fn new_tcp_client<'a>(&'a self, state: &'a TcpClientState) -> etc::TcpClient<'a, 1, 4096, 4096> {
        etc::TcpClient::new(self.net_stack, &state.state)
    }

    pub fn new_udp_socket<'a>(&'a self, data: &'a mut UdpSocketData) -> eu::UdpSocket<'a> {
        eu::UdpSocket::new(
            self.net_stack,
            &mut data.tx_meta,
            &mut data.tx_buffer,
            &mut data.rx_meta,
            &mut data.rx_buffer,
        )
    }

    pub async fn query_dns_a<'a>(&'a self, name: &str) -> Result<IpAddress, &'a str> {
        match self.net_stack.dns_query(name, embassy_net::dns::DnsQueryType::A).await {
            Ok(result) => result.first().map(|addr| addr.clone()).ok_or("No A record found"),
            Err(err) => {
                error!("DNS query failed: {:?}", err);
                Err("DNS query failed")
            }
        }
    }
}
