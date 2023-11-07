use pnet::{
    packet::{icmpv6::echo_request::EchoRequestPacket, ip::IpNextHeaderProtocols::Icmpv6},
    transport::{transport_channel, TransportChannelType, TransportReceiver, TransportSender},
};
use rand::Rng;
use std::net::Ipv6Addr;

#[derive(Debug)]
pub enum IcmPongError {
    CreateSocketError(std::io::Error),
    SendPacketError(std::io::Error),
}

pub enum IcmPongPacketType {
    Ping,
    Pong,
}

pub struct IcmPongPacket<'a> {
    pub version: u8,
    pub packet_type: IcmPongPacketType,
    pub packet_data: &'a [u8; 32],
}

pub struct IcmPongConnection {
    pub peer: Ipv6Addr,
    tx: TransportSender,
    pub rx: TransportReceiver,
    session_id: u64,
}

impl IcmPongConnection {
    pub fn new(peer: Ipv6Addr) -> Result<Self, IcmPongError> {
        let (tx, rx) = match transport_channel(
            1500,
            TransportChannelType::Layer4(pnet::transport::TransportProtocol::Ipv6(Icmpv6)),
        ) {
            Ok((tx, rx)) => (tx, rx),
            Err(error) => return Err(IcmPongError::CreateSocketError(error)),
        };
        Ok(Self {
            peer,
            tx,
            rx,
            session_id: rand::thread_rng().gen(),
        })
    }

    pub fn send_packet(&mut self, packet: IcmPongPacket) -> Result<(), IcmPongError> {
        let packet_payload = &[
            "ICMPong".as_bytes(),
            &packet.version.to_ne_bytes(),
            &self.session_id.to_ne_bytes(),
            &(packet.packet_type as u8).to_ne_bytes(),
            packet.packet_data,
        ]
        .concat();
        let icmp_packet = EchoRequestPacket::new(&packet_payload).unwrap();
        match self.tx.send_to(icmp_packet, self.peer.into()) {
            Ok(_) => Ok(()),
            Err(error) => Err(IcmPongError::SendPacketError(error)),
        }
    }
}
