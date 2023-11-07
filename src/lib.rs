use std::net::Ipv6Addr;

pub enum IcmPongPacketType {
    Ping,
    Pong,
}

pub struct IcmPongPacket {
    version: u64,
    packet_type: IcmPongPacketType,
}

pub struct IcmPongConnection {
    pub remote: Ipv6Addr,
}

impl IcmPongConnection {
    pub fn new(remote: Ipv6Addr) -> Self {
        Self { remote }
    }
}
