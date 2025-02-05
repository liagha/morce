use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use axo_core::{xeprintln, xprint, xprintln, Color};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use crate::{Error, Message, MessageType, Sender};
use crate::server::Server;

pub struct Client {
    pub username: String,
    pub sender: Sender,
}

pub async fn handle_client(stream: TcpStream, clients: Arc<Mutex<HashMap<String, Client>>>, username: String) -> Result<(), Error> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    clients.lock().await.insert(username.clone(), Client { username: username.clone(), sender: tx });

    xprintln!("User '" => Color::Magenta, username => Color::Pink, "' joined the chat." => Color::Magenta);

    let (mut reader, mut writer) = stream.into_split();

    let username_clone = username.clone();
    let clients_clone = Arc::clone(&clients);

    let receive_task = tokio::spawn(async move {
        let mut buffer = [0; 512];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    xprintln!("User '" => Color::BrightRed, username_clone => Color::Crimson, "' disconnected." => Color::BrightRed);

                    clients_clone.lock().await.remove(&username_clone);
                    drop(clients_clone);

                    return Ok(());
                }

                Ok(n) => {
                    let message = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    let clients = clients_clone.lock().await;

                    if message.starts_with('@') {
                        if let Some((target, msg)) = message.split_once(' ') {
                            let target_username = target.trim_start_matches('@');

                            if let Some(client) = clients.get(target_username) {
                                xprintln!("Private message from " => Color::BrightBlue, "'", username_clone, "'", " to " => Color::BrightBlue, "'", target_username, "'", " : ", msg => Color::Blue);

                                let message = Message::from(format!("{} : {}", username_clone, msg).as_str(), MessageType::Private);

                                client.sender.send(message).map_err(|err| Error::Send(err))?;
                            } else {
                                xprintln!("User '" => Color::Red, username_clone, "' tried to message '" => Color::Red, target_username, "', but they are not online." => Color::Red);

                                if let Some(sender) = clients.get(&username_clone) {
                                    let message = Message::from("User not found", MessageType::Private);

                                    sender.sender.send(message).map_err(|err| Error::Send(err))?;
                                }
                            }
                            continue;
                        } else {
                            return Err(Error::MessageConversion)
                        }
                    }

                    xprintln!(username_clone => Color::BrightBlue, " : " => Color::Blue, message);

                    for (_, client) in clients.iter() {
                        if client.username != username_clone {
                            let message = Message::from(format!("{} : {}", username_clone, message).as_str(), MessageType::Public);

                            client.sender.send(message).map_err(|err| Error::Send(err))?;
                        }
                    }
                }
                Err(err) => return Ok(()),
            }
        }
    });

    let send_task : tokio::task::JoinHandle<Result<(), Error>> = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            writer.write_all(&message.as_bytes()?).await.map_err(|err| Error::Write)?;
        }

        Ok(())
    });

    tokio::select! {
        _ = receive_task => {}
        _ = send_task => {}
    }

    Ok(())
}

impl Client {
    pub async fn run_client(server: Server) -> Result<(), Error> {
        let retries = 5;
        let delay = 5;

        xprintln!("Connecting to server..." => Color::Yellow);

        if let Some(stream) = Self::connect_with_retry(&server.address, retries, delay).await {
            let username = loop {
                xprintln!("Enter your username:" => Color::BrightBlue);
                xprint!("> " => Color::DarkGray, "> " => Color::SlateGray, "> ");
                let mut username = String::new();
                std::io::stdin().read_line(&mut username).map_err(|err| Error::Read(err))?;
                let username = username.trim().to_string();

                if username.len() > 3 {
                    break username;
                } else {
                    xeprintln!("Username must be longer than 3 characters." => Color::Orange);
                }
            };

            xprintln!("Sending username: " => Color::BrightBlue, username => Color::Blue);

            let (mut reader, mut writer) = stream.into_split();

            if let Err(e) = writer.write_all(username.as_bytes()).await {
                xeprintln!("Failed to send username: ", e => Color::Orange);
                return Ok(());
            }

            writer.flush().await.map_err(|err| Error::Flush)?;

            xprintln!("Welcome, " => Color::BrightGreen, username => Color::Green, "! Type messages to send to other clients." => Color::BrightGreen);

            let receive_task = tokio::spawn(async move {
                let mut buffer = [0; 512];
                loop {
                    match reader.read(&mut buffer).await {
                        Ok(0) => {
                            xprintln!("Server closed connection. Exiting..." => Color::BrightRed);
                            std::process::exit(0);
                        }
                        Ok(n) => {
                            let response = String::from_utf8_lossy(&buffer[..n]);
                            xprintln!("\n", response);
                            xprint!("> " => Color::DarkGray, "> " => Color::SlateGray, "> ");
                            let _ = tokio::io::stdout().flush();
                        }
                        Err(e) => {
                            xeprintln!("Failed to read from server: ", e => Color::Crimson);
                            break;
                        }
                    }
                }
            });

            let send_task = tokio::spawn(async move {
                loop {
                    xprint!("> " => Color::DarkGray, "> " => Color::SlateGray, "> ");
                    tokio::io::stdout().flush().await?;
                    let mut input = String::new();

                    if std::io::stdin().read_line(&mut input).is_err() {
                        break;
                    }

                    if let Err(e) = writer.write_all(input.as_bytes()).await {
                        xeprintln!("Failed to send data: ", e => Color::Crimson);
                        break;
                    }

                    writer.flush().await?;
                }
                Ok::<_, tokio::io::Error>(())
            });

            tokio::select! {
            _ = receive_task => {}
            result = send_task => {
                if let Err(e) = result {
                    xeprintln!("Send task error: ", e => Color::Crimson ; Debug);
                }
            }
        }
        } else {
            xeprintln!("Failed to connect to server after", retries => Color::Crimson, "retries.");
        }

        Ok(())
    }

    pub async fn connect_with_retry(addr: &String, retries: u32, delay: u64) -> Option<TcpStream> {
        for attempt in 0..retries {
            xprintln!("Attempt " => Color::BrightYellow, attempt + 1 => Color::BrightYellow,": Connecting to " => Color::Yellow, addr => Color::Yellow);

            match TcpStream::connect(addr).await {
                Ok(stream) => {
                    xprintln!("Connected to server at " => Color::BrightGreen, addr => Color::Green);
                    return Some(stream);
                }
                Err(err) => {
                    xprintln!("Failed to connect to server: " => Color::BrightYellow, err => Color::Orange);

                    if attempt < retries - 1 {
                        xprintln!("Retrying in " => Color::Yellow, delay, " seconds..." => Color::Yellow);
                        sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }
        None
    }
}
