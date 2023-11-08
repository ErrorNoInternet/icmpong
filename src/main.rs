mod tui;

use crate::tui::{Field, Game, GAME_TICK_MILLIS, XMAX, XMIN, YMAX, YMIN};
use clap::Parser;
use crossterm::event::{poll, Event, KeyCode};
use crossterm::style::{Color, Print, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, terminal, ExecutableCommand};
use icmpong::{IcmPongConnection, IcmPongPacket, IcmPongPacketType};
use pnet::transport::TransportReceiver;
use pnet::{packet::Packet, transport::icmpv6_packet_iter};
use rand::Rng;
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::{io::Write, net::Ipv6Addr, str::FromStr, time::Duration};

/// The classic game of Pong, in your terminal, over ICMPv6!
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// The IPv6 address of person you want to play ICMPong with
    #[arg(short, long)]
    peer: String,
}

fn main() -> anyhow::Result<()> {
    let arguments = Arguments::parse();
    let ipv6_address = match Ipv6Addr::from_str(&arguments.peer) {
        Ok(ipv6_address) => ipv6_address,
        Err(error) => {
            cleanup()?;
            eprintln!("unable to parse IPv6 address: {error}");
            return Ok(());
        }
    };

    println!("establishing connection with {ipv6_address}...");
    let (connection, mut rx) = match IcmPongConnection::new(ipv6_address) {
        Ok((connection, rx)) => (Arc::new(Mutex::new(connection)), rx),
        Err(error) => {
            cleanup()?;
            eprintln!("unable to create IPv6 socket: {error:?}");
            return Ok(());
        }
    };
    println!("sending Ping packet...");
    match connection
        .lock()
        .unwrap()
        .send_packet(IcmPongPacket::new(IcmPongPacketType::Ping, &[69; 32]))
    {
        Ok(_) => (),
        Err(error) => {
            cleanup()?;
            eprintln!("unable to send Ping packet: {error:?}");
            return Ok(());
        }
    }

    let stop_game = Arc::new(Mutex::new(false));
    let connection_established = Arc::new(Mutex::new(false));
    let peer_client_id = Arc::new(Mutex::new(None));
    let peer_start_game = Arc::new(Mutex::new(false));
    let peer_player = Arc::new(Mutex::new(None));
    let thread_connection = connection.clone();
    let thread_stop_game = stop_game.clone();
    let thread_connnection_established = connection_established.clone();
    let thread_peer_client_id = peer_client_id.clone();
    let thread_peer_start_game = peer_start_game.clone();
    let thread_peer_player = peer_player.clone();
    let connection_thread = std::thread::spawn(move || {
        connection_loop(
            thread_connection,
            &mut rx,
            thread_stop_game,
            thread_connnection_established,
            thread_peer_client_id,
            thread_peer_start_game,
            thread_peer_player,
        )
    });

    while !*connection_established.lock().unwrap() {
        std::thread::sleep(Duration::from_millis(1))
    }
    let self_is_left =
        connection.lock().unwrap().client_id > peer_client_id.lock().unwrap().unwrap();
    println!("{self_is_left}");
    let player1 = Arc::new(Mutex::new(Game::new(
        XMIN + 3,
        (YMAX - YMIN) / 2 - 1,
        4,
        b'X',
    )));
    let player2 = Arc::new(Mutex::new(Game::new(
        XMAX - 4,
        (YMAX - YMIN) / 2 - 1,
        4,
        b'X',
    )));
    *peer_player.lock().unwrap() = Some(if self_is_left {
        player2.clone()
    } else {
        player1.clone()
    });

    let mut self_start_game = false;
    let mut game_started = false;
    let mut ball_moving = false;
    let mut update_screen;
    let mut tick_counter: usize = 0;
    let mut game_tick: Duration;
    let mut ball_velocity: f32 = 0.4;
    let mut field = Field::new();
    let mut ball = Game::new(XMAX / 2, YMAX / 2, 1, b'O');
    let mut round_winner = 0;
    let mut score = [0, 0];

    terminal::enable_raw_mode()?;
    stdout()
        .execute(terminal::Clear(terminal::ClearType::All))?
        .execute(cursor::Hide)?
        .execute(SetBackgroundColor(Color::Black))?
        .execute(SetForegroundColor(Color::White))?
        .flush()?;

    'game_loop: loop {
        game_tick = Duration::from_millis(GAME_TICK_MILLIS);
        tick_counter += 1;
        update_screen = tick_counter == 2;

        if *stop_game.lock().unwrap() {
            break 'game_loop;
        }

        if poll(game_tick)? {
            let event = crossterm::event::read()?;

            if event == Event::Key(KeyCode::Esc.into())
                || event == Event::Key(KeyCode::Char('q').into())
            {
                let _ = connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Disconnect, &[69; 32]));
                break 'game_loop;
            }

            if event == Event::Key(KeyCode::Char(' ').into()) {
                match connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Start, &[69; 32]))
                {
                    Ok(_) => (),
                    Err(error) => {
                        cleanup()?;
                        eprintln!("unable to send Start packet: {error:?}");
                        return Ok(());
                    }
                }
                self_start_game = true;
            }

            if event == Event::Key(KeyCode::Up.into()) {
                if self_is_left {
                    if player1.lock().unwrap().get_ymin() > YMIN + 1 {
                        player1.lock().unwrap().ypos -= 1;
                        let mut data = [69; 32];
                        data[0..2].copy_from_slice(&player1.lock().unwrap().xpos.to_ne_bytes());
                        data[2..4].copy_from_slice(&player1.lock().unwrap().ypos.to_ne_bytes());
                        match connection.lock().unwrap().send_packet(IcmPongPacket::new(
                            IcmPongPacketType::PaddlePosition,
                            &data,
                        )) {
                            Ok(_) => (),
                            Err(error) => {
                                cleanup()?;
                                eprintln!("unable to send PaddlePosition packet: {error:?}");
                                return Ok(());
                            }
                        };
                    }
                } else {
                    if player2.lock().unwrap().get_ymin() > YMIN + 1 {
                        player2.lock().unwrap().ypos -= 1;
                        let mut data = [69; 32];
                        data[0..2].copy_from_slice(&player2.lock().unwrap().xpos.to_ne_bytes());
                        data[2..4].copy_from_slice(&player2.lock().unwrap().ypos.to_ne_bytes());
                        match connection.lock().unwrap().send_packet(IcmPongPacket::new(
                            IcmPongPacketType::PaddlePosition,
                            &data,
                        )) {
                            Ok(_) => (),
                            Err(error) => {
                                cleanup()?;
                                eprintln!("unable to send PaddlePosition packet: {error:?}");
                                return Ok(());
                            }
                        };
                    }
                }
            }

            if event == Event::Key(KeyCode::Down.into()) {
                if self_is_left {
                    if player1.lock().unwrap().get_ymax() < YMAX - 1 {
                        player1.lock().unwrap().ypos += 1;
                        let mut data = [69; 32];
                        data[0..2].copy_from_slice(&player1.lock().unwrap().xpos.to_ne_bytes());
                        data[2..4].copy_from_slice(&player1.lock().unwrap().ypos.to_ne_bytes());
                        match connection.lock().unwrap().send_packet(IcmPongPacket::new(
                            IcmPongPacketType::PaddlePosition,
                            &data,
                        )) {
                            Ok(_) => (),
                            Err(error) => {
                                cleanup()?;
                                eprintln!("unable to send PaddlePosition packet: {error:?}");
                                return Ok(());
                            }
                        };
                    }
                } else {
                    if player2.lock().unwrap().get_ymax() < YMAX - 1 {
                        player2.lock().unwrap().ypos += 1;
                        let mut data = [69; 32];
                        data[0..2].copy_from_slice(&player2.lock().unwrap().xpos.to_ne_bytes());
                        data[2..4].copy_from_slice(&player2.lock().unwrap().ypos.to_ne_bytes());
                        match connection.lock().unwrap().send_packet(IcmPongPacket::new(
                            IcmPongPacketType::PaddlePosition,
                            &data,
                        )) {
                            Ok(_) => (),
                            Err(error) => {
                                cleanup()?;
                                eprintln!("unable to send PaddlePosition packet: {error:?}");
                                return Ok(());
                            }
                        };
                    }
                }
            }
        }

        if ball_moving {
            ball.xf32 += ball.xmov;
            ball.yf32 += ball.ymov;

            ball.xpos = ball.xf32 as u16;
            ball.ypos = ball.yf32 as u16;

            if ball.xpos >= XMAX {
                round_winner = 1;
            }
            if ball.xpos <= XMIN {
                round_winner = 2;
            }
            if ball.get_ymin() <= YMIN || ball.get_ymax() >= YMAX {
                ball.ymov *= -1.0
            }

            if (ball.xpos == player1.lock().unwrap().xpos
                && ball.ypos >= player1.lock().unwrap().get_ymin()
                && ball.ypos <= player1.lock().unwrap().get_ymax())
                || (ball.xpos == player2.lock().unwrap().xpos
                    && ball.ypos >= player2.lock().unwrap().get_ymin()
                    && ball.ypos <= player2.lock().unwrap().get_ymax())
            {
                ball.xmov *= -1.0
            }

            if ball.yf32 > YMAX as f32 {
                ball.yf32 = YMAX as f32 - 1.0;
                ball.ypos = YMAX - 1;
            }
        }

        if update_screen {
            tick_counter = 0;
            field.clear();

            field.write(XMAX / 2 - 5, YMIN, format!(" {:02} ", score[0]).as_str());
            field.write(XMAX / 2 + 2, YMIN, format!(" {:02} ", score[1]).as_str());

            field.draw(&ball);
            field.draw(&player1.lock().unwrap());
            field.draw(&player2.lock().unwrap());

            if !game_started {
                if !self_start_game {
                    let message = "Press SPACE to start the game!";
                    field.write(XMAX / 2 - message.len() as u16 / 2, YMAX - 4, message)
                } else if !*peer_start_game.lock().unwrap() {
                    let message = "Waiting for peer to press SPACE...";
                    field.write(XMAX / 2 - message.len() as u16 / 2, YMAX - 4, message)
                } else {
                    game_started = true;
                    if ball_moving {
                        ball_moving = false;
                        ball = Game::new(XMAX / 2, YMAX / 2, 1, b'O');
                    } else {
                        ball_moving = true;
                        let random_angle = rand::thread_rng().gen_range(-45..45) as f32;
                        ball.xmov = random_angle.cos() * ball_velocity;
                        ball.ymov = random_angle.sin() * ball_velocity;
                    }
                }
            }
        }

        for i in 0..field.field_data.len() {
            let x: u16 = i as u16 % XMAX;
            let y: u16 = i as u16 / XMAX;
            let c: char = field.field_data[i] as char;

            stdout()
                .execute(cursor::MoveTo(x, y))?
                .execute(Print(c))?
                .flush()?;
        }

        if round_winner > 0 {
            score[round_winner - 1] += 1;
            round_winner = 0;
            ball_moving = false;
            ball = Game::new(XMAX / 2, YMAX / 2, 1, b'O');
        }
    }
    connection_thread.join().unwrap();
    stdout().execute(SetBackgroundColor(Color::Reset))?;
    println!("\nquitting!");
    Ok(())
}

fn cleanup() -> anyhow::Result<()> {
    terminal::disable_raw_mode()?;
    stdout().execute(cursor::Show)?.flush()?;
    Ok(())
}

fn connection_loop(
    connection: Arc<Mutex<IcmPongConnection>>,
    rx: &mut TransportReceiver,
    stop_game: Arc<Mutex<bool>>,
    connection_established: Arc<Mutex<bool>>,
    peer_client_id: Arc<Mutex<Option<u32>>>,
    peer_start_game: Arc<Mutex<bool>>,
    peer_player: Arc<Mutex<Option<Arc<Mutex<Game>>>>>,
) {
    let mut client_id = None;
    loop {
        let packet = match icmpv6_packet_iter(rx).next() {
            Ok(packet) => {
                if packet.1 != connection.lock().unwrap().peer {
                    continue;
                } else {
                    packet.0.payload().to_owned()
                }
            }
            Err(error) => {
                let _ = cleanup();
                eprintln!("unable to iterate packets: {error}");
                return;
            }
        };
        if &packet[0..7] == "ICMPong".as_bytes() {
            let packet_version = packet[7];
            if packet_version != icmpong::PROTOCOL_VERSION {
                let _ = cleanup();
                eprintln!("the other player is on a different version of ICMPong!");
                eprintln!(
                    "you are v{}, they are v{packet_version}. please update to the same version.",
                    icmpong::PROTOCOL_VERSION,
                );
                return;
            }
            if packet.len() != 45 {
                let _ = cleanup();
                eprintln!(
                    "invalid packet size received: expected 45, found {}",
                    packet.len()
                );
                return;
            }
            let packet_client_id = u32::from_ne_bytes(match packet[8..12].try_into() {
                Ok(packet_client_id) => packet_client_id,
                Err(error) => {
                    eprintln!("unable to deserialize peer client id: {error}");
                    return;
                }
            });
            if packet_client_id == connection.lock().unwrap().client_id {
                continue;
            }
            let packet_type: IcmPongPacketType =
                match num_traits::FromPrimitive::from_u8(packet[12]) {
                    Some(packet_type) => packet_type,
                    None => {
                        let _ = cleanup();
                        eprintln!("unknown packet type received ({})", packet[12]);
                        return;
                    }
                };
            let packet_data = &packet[13..45];

            if packet_type == IcmPongPacketType::Disconnect {
                let _ = connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Disconnect, &[69; 32]));
                let _ = cleanup();
                *stop_game.lock().unwrap() = true;
                return;
            }

            if packet_type == IcmPongPacketType::Ping {
                println!("received Ping packet from peer! sending Ready packet...");
                match connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Ready, &[69; 32]))
                {
                    Ok(_) => (),
                    Err(error) => {
                        let _ = cleanup();
                        eprintln!("unable to send Ready packet: {error:?}");
                        return;
                    }
                };
            }
            if packet_type == IcmPongPacketType::Ready && client_id.is_none() {
                println!("received Ready packet from peer! echoing...");
                match connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Ready, &[69; 32]))
                {
                    Ok(_) => (),
                    Err(error) => {
                        let _ = cleanup();
                        eprintln!("unable to send Ready packet: {error:?}");
                        return;
                    }
                };
                client_id = Some(packet_client_id);
                *peer_client_id.lock().unwrap() = client_id;
                println!(
                    "starting game with {} (peer client id: {packet_client_id})...",
                    connection.lock().unwrap().peer
                );
                *connection_established.lock().unwrap() = true;
            }

            if client_id.is_some() {
                if packet_type == IcmPongPacketType::Start {
                    *peer_start_game.lock().unwrap() = true;
                } else if packet_type == IcmPongPacketType::PaddlePosition {
                    let player = peer_player.lock().unwrap();
                    match player.to_owned() {
                        Some(player) => {
                            player.lock().unwrap().xpos =
                                u16::from_ne_bytes(packet_data[0..2].try_into().unwrap());
                            player.lock().unwrap().ypos =
                                u16::from_ne_bytes(packet_data[2..4].try_into().unwrap());
                        }
                        None => (),
                    }
                }
            }
        }
    }
}
