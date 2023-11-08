use clap::Parser;
use icmpong::{IcmPongConnection, IcmPongPacket, IcmPongPacketType};
use pnet::{packet::Packet, transport::icmpv6_packet_iter};
use std::{net::Ipv6Addr, str::FromStr};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// The IPv6 address of person you want to play pong with
    #[arg(short, long)]
    peer: String,
}

fn main() {
    let arguments = Arguments::parse();

    let ipv6_address = match Ipv6Addr::from_str(&arguments.peer) {
        Ok(ipv6_address) => ipv6_address,
        Err(error) => {
            eprintln!("unable to parse IPv6 address: {error}");
            return;
        }
    };

    println!("establishing connection with {ipv6_address}...");
    let mut connection = match IcmPongConnection::new(ipv6_address) {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("unable to create IPv6 socket: {error:?}");
            return;
        }
    };
    println!("sending ICMPong PING...");
    match connection.send_packet(IcmPongPacket::new(IcmPongPacketType::Ping, &[69; 32])) {
        Ok(_) => (),
        Err(error) => {
            eprintln!("unable to send PING: {error:?}");
            return;
        }
    }

    let mut peer_client_id = None;
    loop {
        let packet = icmpv6_packet_iter(&mut connection.rx)
            .next()
            .unwrap()
            .0
            .payload()
            .to_owned();
        if &packet[0..7] == "ICMPong".as_bytes() {
            let packet_version = packet[7];
            if packet_version != icmpong::PROTOCOL_VERSION {
                eprintln!("the other player is on a different version of ICMPong!");
                eprintln!(
                    "you are v{}, they are v{packet_version}. please update to the same version.",
                    icmpong::PROTOCOL_VERSION,
                );
                return;
            }
            if packet.len() != 45 {
                eprintln!(
                    "invalid packet size received: expected 45, found {}",
                    packet.len()
                );
                return;
            }
            let client_id = u32::from_ne_bytes(match packet[8..12].try_into() {
                Ok(peer_client_id) => peer_client_id,
                Err(error) => {
                    eprintln!("unable to deserialize peer client id: {error}");
                    return;
                }
            });
            if client_id == connection.client_id {
                continue;
            }
            let packet_type: IcmPongPacketType =
                match num_traits::FromPrimitive::from_u8(packet[12]) {
                    Some(packet_type) => packet_type,
                    None => {
                        eprintln!("unknown packet type received ({})", packet[12]);
                        return;
                    }
                };
            let packet_data = &packet[13..45];

            if packet_type == IcmPongPacketType::Ping {
                println!("received PING from peer! sending READY...");
                match connection
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Ready, &[69; 32]))
                {
                    Ok(_) => (),
                    Err(error) => {
                        eprintln!("unable to send READY: {error:?}");
                        return;
                    }
                };
            }
            if packet_type == IcmPongPacketType::Ready {
                if peer_client_id.is_none() {
                    println!("received READY from peer! echoing...");
                    match connection
                        .send_packet(IcmPongPacket::new(IcmPongPacketType::Ready, &[69; 32]))
                    {
                        Ok(_) => (),
                        Err(error) => {
                            eprintln!("unable to send READY: {error:?}");
                            return;
                        }
                    };
                    peer_client_id = Some(client_id);
                    println!(
                        "starting game with {} (id: {client_id})...",
                        connection.peer
                    );
                }
            }
        }
    }
}
