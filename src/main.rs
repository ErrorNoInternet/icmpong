mod tui;

use crate::tui::{
    Field, GameObject, GAME_TICK_MILLISECONDS, X_MAXIMUM, X_MINIMUM, Y_MAXIMUM, Y_MINIMUM,
};
use clap::Parser;
use crossterm::event::{poll, Event, KeyCode};
use crossterm::style::{Color, Print, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, terminal, ExecutableCommand};
use icmpong::{IcmPongConnection, IcmPongError, IcmPongPacket, IcmPongPacketType};
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

    /// The name you want the other person to see
    #[arg(short, long)]
    name: Option<String>,

    /// The initial ball velocity (will slowly increase after each bounce)
    #[arg(short, long, default_value_t = 0.6)]
    ball_velocity: f32,
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
    if let Some(ref name) = arguments.name {
        if name.len() > 30 {
            eprintln!("your name must not be longer than 30 characters!");
            return Ok(());
        }
    }

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
        .send_packet(IcmPongPacket::new(IcmPongPacketType::Ping, &[0; 32]))
    {
        Ok(_) => (),
        Err(error) => {
            cleanup()?;
            eprintln!("unable to send Ping packet: {error:?}");
            return Ok(());
        }
    }

    let self_name = arguments.name.clone();
    let stop_game = Arc::new(Mutex::new(false));
    let connection_established = Arc::new(Mutex::new(false));
    let peer_client_id = Arc::new(Mutex::new(None));
    let peer_start_game = Arc::new(Mutex::new(false));
    let ball = Arc::new(Mutex::new(GameObject::new(
        X_MAXIMUM / 2,
        Y_MAXIMUM / 2,
        1,
        b'O',
    )));
    let peer_player = Arc::new(Mutex::new(None));
    let peer_name = Arc::new(Mutex::new(None));
    let score = Arc::new(Mutex::new([0, 0]));
    let thread_connection = connection.clone();
    let thread_stop_game = stop_game.clone();
    let thread_connnection_established = connection_established.clone();
    let thread_peer_client_id = peer_client_id.clone();
    let thread_peer_start_game = peer_start_game.clone();
    let thread_ball = ball.clone();
    let thread_peer_player = peer_player.clone();
    let thread_peer_name = peer_name.clone();
    let thread_score = score.clone();
    let connection_thread = std::thread::spawn(move || {
        connection_loop(
            self_name,
            thread_connection,
            &mut rx,
            thread_stop_game,
            thread_connnection_established,
            thread_peer_client_id,
            thread_peer_start_game,
            thread_ball,
            thread_peer_player,
            thread_peer_name,
            thread_score,
        )
    });

    while !*connection_established.lock().unwrap() {
        std::thread::sleep(Duration::from_millis(1))
    }
    let self_is_host =
        connection.lock().unwrap().client_id > peer_client_id.lock().unwrap().unwrap();
    let player1 = Arc::new(Mutex::new(GameObject::new(
        X_MINIMUM + 3,
        (Y_MAXIMUM - Y_MINIMUM) / 2 - 1,
        4,
        b'X',
    )));
    let player2 = Arc::new(Mutex::new(GameObject::new(
        X_MAXIMUM - 4,
        (Y_MAXIMUM - Y_MINIMUM) / 2 - 1,
        4,
        b'X',
    )));
    *peer_player.lock().unwrap() = Some(if self_is_host {
        player2.clone()
    } else {
        player1.clone()
    });

    let mut field = Field::new();
    let mut game_started = false;
    let mut round_winner = 0;
    let mut self_start_game = false;
    let mut tick_counter = 0;
    let mut bounces = 0;

    terminal::enable_raw_mode()?;
    stdout()
        .execute(terminal::Clear(terminal::ClearType::All))?
        .execute(cursor::Hide)?
        .execute(SetBackgroundColor(Color::Black))?
        .execute(SetForegroundColor(Color::White))?
        .flush()?;

    let game_tick = Duration::from_millis(GAME_TICK_MILLISECONDS);
    'game_loop: loop {
        tick_counter += 1;

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
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Disconnect, &[0; 32]));
                break 'game_loop;
            }

            if event == Event::Key(KeyCode::Char(' ').into()) {
                match connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Start, &[0; 32]))
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

            if self_start_game {
                if event == Event::Key(KeyCode::Up.into()) {
                    if self_is_host {
                        if player1.lock().unwrap().get_ymin() > Y_MINIMUM + 1 {
                            player1.lock().unwrap().y_position -= 1;
                            let mut data = [0; 32];
                            data[0..2]
                                .copy_from_slice(&player1.lock().unwrap().x_position.to_ne_bytes());
                            data[2..4]
                                .copy_from_slice(&player1.lock().unwrap().y_position.to_ne_bytes());
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
                        if player2.lock().unwrap().get_ymin() > Y_MINIMUM + 1 {
                            player2.lock().unwrap().y_position -= 1;
                            let mut data = [0; 32];
                            data[0..2]
                                .copy_from_slice(&player2.lock().unwrap().x_position.to_ne_bytes());
                            data[2..4]
                                .copy_from_slice(&player2.lock().unwrap().y_position.to_ne_bytes());
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
                    if self_is_host {
                        if player1.lock().unwrap().get_ymax() < Y_MAXIMUM - 1 {
                            player1.lock().unwrap().y_position += 1;
                            let mut data = [0; 32];
                            data[0..2]
                                .copy_from_slice(&player1.lock().unwrap().x_position.to_ne_bytes());
                            data[2..4]
                                .copy_from_slice(&player1.lock().unwrap().y_position.to_ne_bytes());
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
                        if player2.lock().unwrap().get_ymax() < Y_MAXIMUM - 1 {
                            player2.lock().unwrap().y_position += 1;
                            let mut data = [0; 32];
                            data[0..2]
                                .copy_from_slice(&player2.lock().unwrap().x_position.to_ne_bytes());
                            data[2..4]
                                .copy_from_slice(&player2.lock().unwrap().y_position.to_ne_bytes());
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
        }

        if game_started {
            let mut xf32 = ball.lock().unwrap().xf32;
            xf32 += ball.lock().unwrap().x_movement;
            ball.lock().unwrap().xf32 = xf32;
            let mut yf32 = ball.lock().unwrap().yf32;
            yf32 += ball.lock().unwrap().y_movement;
            ball.lock().unwrap().yf32 = yf32;
            ball.lock().unwrap().x_position = xf32 as u16;
            ball.lock().unwrap().y_position = yf32 as u16;

            if self_is_host {
                if ball.lock().unwrap().x_position >= X_MAXIMUM {
                    round_winner = 1;
                }
                if ball.lock().unwrap().x_position <= X_MINIMUM {
                    round_winner = 2;
                }
            }
            if ball.lock().unwrap().get_ymin() <= Y_MINIMUM
                || ball.lock().unwrap().get_ymax() >= Y_MAXIMUM
            {
                ball.lock().unwrap().y_movement *= -1.0;
                if self_is_host {
                    bounces += 1;
                    if bounces % 5 == 0 && ball.lock().unwrap().y_movement.abs() <= 1.0 {
                        ball.lock().unwrap().x_movement *= 1.1;
                        ball.lock().unwrap().y_movement *= 1.1;
                    }
                    match synchronize_ball(&connection, &ball) {
                        Ok(_) => (),
                        Err(error) => {
                            cleanup()?;
                            eprintln!("unable to send BallUpdate packet: {error:?}");
                            return Ok(());
                        }
                    }
                }
            }

            if (ball.lock().unwrap().x_position == player1.lock().unwrap().x_position
                && ball.lock().unwrap().y_position >= player1.lock().unwrap().get_ymin()
                && ball.lock().unwrap().y_position <= player1.lock().unwrap().get_ymax())
                || (ball.lock().unwrap().x_position == player2.lock().unwrap().x_position
                    && ball.lock().unwrap().y_position >= player2.lock().unwrap().get_ymin()
                    && ball.lock().unwrap().y_position <= player2.lock().unwrap().get_ymax())
            {
                ball.lock().unwrap().x_movement *= -1.0;
                if self_is_host {
                    match synchronize_ball(&connection, &ball) {
                        Ok(_) => (),
                        Err(error) => {
                            cleanup()?;
                            eprintln!("unable to send BallUpdate packet: {error:?}");
                            return Ok(());
                        }
                    }
                }
            }

            if ball.lock().unwrap().yf32 > Y_MAXIMUM as f32 {
                ball.lock().unwrap().yf32 = Y_MAXIMUM as f32 - 1.0;
                ball.lock().unwrap().y_position = Y_MAXIMUM - 1;
                if self_is_host {
                    match synchronize_ball(&connection, &ball) {
                        Ok(_) => (),
                        Err(error) => {
                            cleanup()?;
                            eprintln!("unable to send BallUpdate packet: {error:?}");
                            return Ok(());
                        }
                    }
                }
            }
        }

        if tick_counter == 2 {
            tick_counter = 0;
            field.clear();

            field.write(
                X_MAXIMUM / 2 - 5,
                Y_MINIMUM,
                format!(" {:02} ", score.lock().unwrap()[0]).as_str(),
            );
            field.write(
                X_MAXIMUM / 2 + 2,
                Y_MINIMUM,
                format!(" {:02} ", score.lock().unwrap()[1]).as_str(),
            );
            if let Some(ref name) = arguments.name {
                field.write(X_MINIMUM, Y_MINIMUM, &name);
            }
            if let Some(peer_name) = peer_name.lock().unwrap().to_owned() {
                field.write(X_MAXIMUM - peer_name.len() as u16, Y_MINIMUM, &peer_name);
            }

            field.draw(&ball.lock().unwrap());
            field.draw(&player1.lock().unwrap());
            field.draw(&player2.lock().unwrap());

            if !game_started {
                if !self_start_game {
                    let message = "Press SPACE to start the game!";
                    field.write(
                        X_MAXIMUM / 2 - message.len() as u16 / 2,
                        Y_MAXIMUM - 4,
                        message,
                    )
                } else if !*peer_start_game.lock().unwrap() {
                    let message = "Waiting for peer to press SPACE...";
                    field.write(
                        X_MAXIMUM / 2 - message.len() as u16 / 2,
                        Y_MAXIMUM - 4,
                        message,
                    )
                } else {
                    game_started = true;
                    if self_is_host {
                        let random_angle = rand::thread_rng().gen_range(-45..45) as f32;
                        ball.lock().unwrap().x_movement =
                            random_angle.cos() * arguments.ball_velocity;
                        ball.lock().unwrap().y_movement =
                            random_angle.sin() * arguments.ball_velocity;
                        match synchronize_ball(&connection, &ball) {
                            Ok(_) => (),
                            Err(error) => {
                                cleanup()?;
                                eprintln!("unable to send BallUpdate packet: {error:?}");
                                return Ok(());
                            }
                        }
                    }
                }
            }

            for i in 0..field.field_data.len() {
                let x: u16 = i as u16 % X_MAXIMUM;
                let y: u16 = i as u16 / X_MAXIMUM;
                let c: char = field.field_data[i] as char;

                stdout()
                    .execute(cursor::MoveTo(x, y))?
                    .execute(Print(c))?
                    .flush()?;
            }
        }

        if round_winner > 0 {
            score.lock().unwrap()[round_winner - 1] += 1;
            round_winner = 0;
            *ball.lock().unwrap() = GameObject::new(X_MAXIMUM / 2, Y_MAXIMUM / 2, 1, b'O');
            if self_is_host {
                bounces = 0;
                let mut data = [0; 32];
                data[0..4].copy_from_slice(&score.lock().unwrap()[0].to_ne_bytes());
                data[4..8].copy_from_slice(&score.lock().unwrap()[1].to_ne_bytes());
                match connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::ScoreUpdate, &data))
                {
                    Ok(_) => (),
                    Err(error) => {
                        cleanup()?;
                        eprintln!("unable to send ScoreUpdate packet: {error:?}");
                        return Ok(());
                    }
                }

                let random_angle = rand::thread_rng().gen_range(-45..45) as f32;
                ball.lock().unwrap().x_movement = random_angle.cos() * arguments.ball_velocity;
                ball.lock().unwrap().y_movement = random_angle.sin() * arguments.ball_velocity;
                match synchronize_ball(&connection, &ball) {
                    Ok(_) => (),
                    Err(error) => {
                        cleanup()?;
                        eprintln!("unable to send BallUpdate packet: {error:?}");
                        return Ok(());
                    }
                }
            }
        }
    }
    connection_thread.join().unwrap();
    stdout().execute(SetBackgroundColor(Color::Reset))?;
    println!("\nquitting!");
    Ok(())
}

fn synchronize_ball(
    connection: &Arc<Mutex<IcmPongConnection>>,
    ball: &Arc<Mutex<GameObject>>,
) -> Result<(), IcmPongError> {
    let mut data = [0; 32];
    data[0..2].copy_from_slice(&ball.lock().unwrap().x_position.to_ne_bytes());
    data[2..4].copy_from_slice(&ball.lock().unwrap().y_position.to_ne_bytes());
    data[4..8].copy_from_slice(&ball.lock().unwrap().x_movement.to_ne_bytes());
    data[8..12].copy_from_slice(&ball.lock().unwrap().y_movement.to_ne_bytes());
    data[12..16].copy_from_slice(&ball.lock().unwrap().xf32.to_ne_bytes());
    data[16..20].copy_from_slice(&ball.lock().unwrap().yf32.to_ne_bytes());
    connection
        .lock()
        .unwrap()
        .send_packet(IcmPongPacket::new(IcmPongPacketType::BallUpdate, &data))?;
    Ok(())
}

fn cleanup() -> anyhow::Result<()> {
    terminal::disable_raw_mode()?;
    stdout().execute(cursor::Show)?.flush()?;
    Ok(())
}

fn connection_loop(
    name: Option<String>,
    connection: Arc<Mutex<IcmPongConnection>>,
    rx: &mut TransportReceiver,
    stop_game: Arc<Mutex<bool>>,
    connection_established: Arc<Mutex<bool>>,
    peer_client_id: Arc<Mutex<Option<u32>>>,
    peer_start_game: Arc<Mutex<bool>>,
    ball: Arc<Mutex<GameObject>>,
    peer_player: Arc<Mutex<Option<Arc<Mutex<GameObject>>>>>,
    peer_name: Arc<Mutex<Option<String>>>,
    score: Arc<Mutex<[u32; 2]>>,
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
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Disconnect, &[0; 32]));
                let _ = cleanup();
                *stop_game.lock().unwrap() = true;
                return;
            } else if packet_type == IcmPongPacketType::Ping {
                println!("received Ping packet from peer! sending Ready packet...");
                let data = &mut [0u8; 32];
                if let Some(ref name) = name {
                    data[0] = name.len() as u8;
                    data[1..name.len() + 1].copy_from_slice(name.clone().as_bytes());
                }
                match connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Ready, &data))
                {
                    Ok(_) => (),
                    Err(error) => {
                        let _ = cleanup();
                        eprintln!("unable to send Ready packet: {error:?}");
                        return;
                    }
                };
            } else if packet_type == IcmPongPacketType::Ready && client_id.is_none() {
                println!("received Ready packet from peer! echoing...");
                if packet_data[0] != 0 {
                    let name_length = packet_data[0] as usize;
                    *peer_name.lock().unwrap() = Some(
                        std::str::from_utf8(&packet_data[1..name_length + 1])
                            .unwrap()
                            .to_string(),
                    )
                }

                let data = &mut [0u8; 32];
                if let Some(ref name) = name {
                    data[0] = name.len() as u8;
                    data[1..name.len() + 1].copy_from_slice(name.clone().as_bytes());
                }
                match connection
                    .lock()
                    .unwrap()
                    .send_packet(IcmPongPacket::new(IcmPongPacketType::Ready, &data))
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
                            player.lock().unwrap().x_position =
                                u16::from_ne_bytes(packet_data[0..2].try_into().unwrap());
                            player.lock().unwrap().y_position =
                                u16::from_ne_bytes(packet_data[2..4].try_into().unwrap());
                        }
                        None => (),
                    }
                } else if packet_type == IcmPongPacketType::BallUpdate {
                    ball.lock().unwrap().x_position =
                        u16::from_ne_bytes(packet_data[0..2].try_into().unwrap());
                    ball.lock().unwrap().y_position =
                        u16::from_ne_bytes(packet_data[2..4].try_into().unwrap());
                    ball.lock().unwrap().x_movement =
                        f32::from_ne_bytes(packet_data[4..8].try_into().unwrap());
                    ball.lock().unwrap().y_movement =
                        f32::from_ne_bytes(packet_data[8..12].try_into().unwrap());
                    ball.lock().unwrap().xf32 =
                        f32::from_ne_bytes(packet_data[12..16].try_into().unwrap());
                    ball.lock().unwrap().yf32 =
                        f32::from_ne_bytes(packet_data[16..20].try_into().unwrap());
                } else if packet_type == IcmPongPacketType::ScoreUpdate {
                    score.lock().unwrap()[0] =
                        u32::from_ne_bytes(packet_data[0..4].try_into().unwrap());
                    score.lock().unwrap()[1] =
                        u32::from_ne_bytes(packet_data[4..8].try_into().unwrap());
                }
            }
        }
    }
}
