pub mod lib;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// The IPv6 address of person you want to play pong with
    #[arg(short, long)]
    peer: String,
}

fn main() {
    let arguments = Arguments::parse();
}
