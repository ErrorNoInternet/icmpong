pub mod lib;

use clap::Parser;
use lib::IcmPongConnection;
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
    let connection = match IcmPongConnection::new(ipv6_address) {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("unable to create IPv6 socket: {error:?}");
            return;
        }
    };
}
