use clap::Parser;
use icmpong::{IcmPongConnection, IcmPongPacket, IcmPongPacketType};
use std::{net::Ipv6Addr, str::FromStr};

const PROTOCOL_VERSION: u8 = 0;

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
    match connection.send_packet(IcmPongPacket {
        version: PROTOCOL_VERSION,
        packet_type: IcmPongPacketType::Ping,
        packet_data: &[0; 32],
    }) {
        Ok(_) => (),
        Err(error) => {
            eprintln!("unable to send PING: {error:?}");
            return;
        }
    }
}
