use embassy_net::Stack;
use embassy_net::tcp::client as etc;

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

impl Net {
    pub fn new(net_stack: Stack<'static>) -> Self {
        Net { net_stack }
    }

    pub fn new_tcp_client<'a>(
        &'a self,
        state: &'a TcpClientState,
    ) -> etc::TcpClient<'a, 1, 4096, 4096> {
        etc::TcpClient::new(self.net_stack, &state.state)
    }
}
