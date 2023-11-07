use rand::Rng;
use std::net::Ipv6Addr;

pub enum IcmPongPacketType {
    Ping,
    Pong,
}

pub struct IcmPongPacket {
    version: u64,
    session_id: u64,
    packet_type: IcmPongPacketType,
}

pub struct IcmPongConnection {
    pub peer: Ipv6Addr,
    session_id: u64,
}

impl IcmPongConnection {
    pub fn new(peer: Ipv6Addr) -> Self {
        Self {
            peer,
            session_id: rand::thread_rng().gen(),
        }
    }
}
