use rand::Rng;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::Ipv6Addr;

#[derive(Debug)]
pub enum IcmPongError {
    CreateSocketError(std::io::Error),
}

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
    socket: Socket,
    session_id: u64,
}

impl IcmPongConnection {
    pub fn new(peer: Ipv6Addr) -> Result<Self, IcmPongError> {
        let socket = match Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::ICMPV6)) {
            Ok(socket) => socket,
            Err(error) => return Err(IcmPongError::CreateSocketError(error)),
        };
        Ok(Self {
            peer,
            socket,
            session_id: rand::thread_rng().gen(),
        })
    }
}
