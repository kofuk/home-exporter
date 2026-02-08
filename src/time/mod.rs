extern crate alloc;

use crate::networking::{NetworkingStack, UdpSocketData};
use alloc::rc::Rc;
use defmt::*;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};

const NTP_SERVER: &str = "pool.ntp.org";

mod ntp {
    #[repr(C, packed)]
    pub struct Packet {
        li_vn_mode: u8,
        stratum: u8,
        poll: u8,
        precision: u8,
        root_delay: u32,
        root_dispersion: u32,
        ref_id: u32,
        ref_timestamp: u64,
        orig_timestamp: u64,
        recv_timestamp: u64,
        transmit_timestamp: u64,
    }

    impl Default for Packet {
        fn default() -> Self {
            Self {
                li_vn_mode: 0x1b, // LI=0, VN=3, Mode=3 (client)
                stratum: 0,
                poll: 0,
                precision: 0,
                root_delay: 0,
                root_dispersion: 0,
                ref_id: 0,
                ref_timestamp: 0,
                orig_timestamp: 0,
                recv_timestamp: 0,
                transmit_timestamp: 0,
            }
        }
    }

    impl Packet {
        pub fn to_bytes(&self) -> [u8; 48] {
            let mut bytes = [0u8; 48];
            bytes[0] = self.li_vn_mode;
            bytes
        }

        pub fn from_bytes(bytes: &[u8]) -> Result<Self, &str> {
            if bytes.len() < 48 {
                return Err("Buffer too short");
            }

            Ok(Self {
                li_vn_mode: bytes[0],
                stratum: bytes[1],
                poll: bytes[2],
                precision: bytes[3],
                root_delay: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                root_dispersion: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                ref_id: u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
                ref_timestamp: u64::from_be_bytes([
                    bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
                ]),
                orig_timestamp: u64::from_be_bytes([
                    bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
                ]),
                recv_timestamp: u64::from_be_bytes([
                    bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38], bytes[39],
                ]),
                transmit_timestamp: u64::from_be_bytes([
                    bytes[40], bytes[41], bytes[42], bytes[43], bytes[44], bytes[45], bytes[46], bytes[47],
                ]),
            })
        }

        pub fn get_unix_time(&self) -> u64 {
            // NTP timestamp to Unix timestamp
            // NTP epoch: 1900-01-01, Unix epoch: 1970-01-01
            const NTP_UNIX_OFFSET: u64 = 2208988800;

            let seconds = (self.transmit_timestamp >> 32) as u64;
            seconds.saturating_sub(NTP_UNIX_OFFSET)
        }
    }
}

pub struct Time {
    networking_stack: Rc<Mutex<NoopRawMutex, NetworkingStack>>,
    base_timestamp: u64,
    seconds_since_boot: u64,
}

impl Time {
    pub fn new(networking_stack: Rc<Mutex<NoopRawMutex, NetworkingStack>>) -> Self {
        Time {
            networking_stack,
            base_timestamp: 0,
            seconds_since_boot: 0,
        }
    }

    pub fn now(&self) -> u64 {
        let base_ticks = self.seconds_since_boot;
        let current_ticks = embassy_time::Instant::now().as_secs();
        let elapsed_secs = current_ticks - base_ticks;

        self.base_timestamp + elapsed_secs
    }

    pub async fn sync_once(&mut self) -> Result<(), ()> {
        let net = &self.networking_stack.lock().await.net;
        let server_addr = net.query_dns_a(NTP_SERVER).await.map_err(|err| {
            error!("Could not lookup NTP server address: {:?}", err);
            ()
        })?;

        let mut udp_data = UdpSocketData::default();
        let mut udp_socket = net.new_udp_socket(&mut udp_data);

        match udp_socket.bind(0) {
            Err(err) => {
                error!("Could not bind UDP socket: {:?}", err);
                return Err(());
            }
            _ => {}
        };

        let ntp_packet = ntp::Packet::default().to_bytes();
        match udp_socket.send_to(&ntp_packet, (server_addr, 123)).await {
            Err(err) => {
                error!("Could not send NTP packet: {:?}", err);
                return Err(());
            }
            _ => {}
        }

        let mut response = [0u8; 512];
        let n_read = match udp_socket.recv_from(&mut response).await {
            Ok((n, ..)) => n,
            Err(err) => {
                error!("Could not receive NTP packet: {:?}", err);
                return Err(());
            }
        };

        let seconds_since_boot = embassy_time::Instant::now().as_secs();

        let unix_timestamp = match ntp::Packet::from_bytes(&response[..n_read]) {
            Ok(packet) => packet.get_unix_time(),
            Err(err) => {
                error!("Could not parse NTP packet: {:?}", err);
                return Err(());
            }
        };

        self.base_timestamp = unix_timestamp;
        self.seconds_since_boot = seconds_since_boot;

        Ok(())
    }

    pub async fn do_sync_loop(instance: Rc<Mutex<NoopRawMutex, Time>>) {
        loop {
            match instance.lock().await.sync_once().await {
                Err(_) => error!("Sync once failed"),
                _ => {}
            }

            embassy_time::Timer::after_secs(60 * 60).await;
        }
    }
}
