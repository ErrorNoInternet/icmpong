use num_derive::FromPrimitive;
use pnet::{
    packet::{
        icmpv6::{echo_request::MutableEchoRequestPacket, Icmpv6Types},
        ip::IpNextHeaderProtocols::Icmpv6,
    },
    transport::{transport_channel, TransportChannelType, TransportReceiver, TransportSender},
};
use rand::Rng;
use std::net::Ipv6Addr;

pub const PROTOCOL_VERSION: u8 = 0;

#[derive(Debug)]
pub enum IcmPongError {
    CreateSocketError(std::io::Error),
    SendPacketError(std::io::Error),
}

#[derive(Debug, FromPrimitive, PartialEq)]
pub enum IcmPongPacketType {
    Ping,
    Ready,
    Start,
    PaddlePosition,
}

pub struct IcmPongPacket<'a> {
    pub version: u8,
    pub packet_type: IcmPongPacketType,
    pub packet_data: &'a [u8; 32],
}

impl<'a> IcmPongPacket<'a> {
    pub fn new(packet_type: IcmPongPacketType, packet_data: &'a [u8; 32]) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            packet_type,
            packet_data,
        }
    }
}

pub struct IcmPongConnection {
    pub peer: Ipv6Addr,
    pub tx: TransportSender,
    pub client_id: u32,
}

impl IcmPongConnection {
    pub fn new(peer: Ipv6Addr) -> Result<(Self, TransportReceiver), IcmPongError> {
        let (tx, rx) = match transport_channel(
            1500,
            TransportChannelType::Layer4(pnet::transport::TransportProtocol::Ipv6(Icmpv6)),
        ) {
            Ok((tx, rx)) => (tx, rx),
            Err(error) => return Err(IcmPongError::CreateSocketError(error)),
        };
        Ok((
            Self {
                peer,
                tx,
                client_id: rand::thread_rng().gen(),
            },
            rx,
        ))
    }

    pub fn send_packet(&mut self, packet: IcmPongPacket) -> Result<(), IcmPongError> {
        let mut packet_payload = [
            "....ICMPong".as_bytes(),
            &packet.version.to_ne_bytes(),
            &self.client_id.to_ne_bytes(),
            &(packet.packet_type as u8).to_ne_bytes(),
            packet.packet_data,
        ]
        .concat();
        let mut icmp_packet = MutableEchoRequestPacket::new(&mut packet_payload).unwrap();
        icmp_packet.set_icmpv6_type(Icmpv6Types::EchoRequest);
        match self.tx.send_to(icmp_packet, self.peer.into()) {
            Ok(_) => Ok(()),
            Err(error) => Err(IcmPongError::SendPacketError(error)),
        }
    }
}
