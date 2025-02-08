use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use axo_core::{xeprintln, xprintln, Color};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use crate::{Address, Error, Sender, Message};
use crate::message::MessageType;


pub struct Client {
    pub username: String,
    pub sender: Sender,
}

pub async fn handle_client(stream: TcpStream, clients: Arc<Mutex<HashMap<String, Client>>>, username: String) -> Result<(), Error> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    clients.lock().await.insert(username.clone(), Client { username: username.clone(), sender: tx });

    xprintln!("User '" => Color::Magenta, username => Color::Pink, "' joined the chat." => Color::Magenta);

    let (mut reader, mut writer) = stream.into_split();

    let username_for_send = username.clone();
    let clients_for_send = Arc::clone(&clients);

    let receive_task = tokio::spawn(async move {
        let mut buffer = [0; 512];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    xprintln!("User '" => Color::BrightRed, username => Color::Crimson, "' disconnected." => Color::BrightRed);

                    clients.lock().await.remove(&username);
                    return Ok(());
                }
                Ok(n) => {
                    let message = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    let clients = clients.lock().await;

                    if message.starts_with('@') {
                        if let Some((target, msg)) = message.split_once(' ') {
                            let target_username = target.trim_start_matches('@');

                            if let Some(client) = clients.get(target_username) {
                                xprintln!("Private message from " => Color::BrightBlue, "'", username, "'", " to " => Color::BrightBlue, "'", target_username, "'", " : ", msg => Color::Blue);

                                let message = Message::from(msg, username.clone(), MessageType::Private);
                                client.sender.send(message).map_err(|err| Error::Send(err))?;
                            } else {
                                xprintln!("User '" => Color::Red, username, "' tried to message '" => Color::Red, target_username, "', but they are not online." => Color::Red);

                                if let Some(sender) = clients.get(&username) {
                                    let message = Message::from("User not found", "Server".to_string(), MessageType::Private);
                                    sender.sender.send(message).map_err(|err| Error::Send(err))?;
                                }
                            }
                            continue;
                        } else {
                            return Err(Error::MessageConversion);
                        }
                    }

                    xprintln!(username => Color::BrightBlue, " : " => Color::Blue, message);

                    for (_, client) in clients.iter() {
                        if client.username != username {
                            let message = Message::from(message.as_str(), username.clone(), MessageType::Public);
                            client.sender.send(message).map_err(|err| Error::Send(err))?;
                        }
                    }
                }
                Err(err) => {
                    xeprintln!("Error reading buffer: ", err);
                    clients.lock().await.remove(&username);
                    return Ok(());
                }
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(err) = writer.write_all(&message.as_bytes()?).await {
                xeprintln!("Failed to send message: ", err => Color::Crimson);
                clients_for_send.lock().await.remove(&username_for_send);
                return Err(Error::Write);
            }
        }
        Ok(())
    });

    tokio::select! {
        result = receive_task => {
            if let Err(err) = result {
                xeprintln!("Receive task error: ", err => Color::Crimson);
            }
        }
        result = send_task => {
            if let Err(err) = result {
                xeprintln!("Send task error: ", err => Color::Crimson);
            }
        }
    }

    Ok(())
}

impl Client {
    pub async fn run_client(server: Address) -> Result<(), Error> {
        let retries = 5;
        let delay = 5;

        xprintln!("Connecting to server..." => Color::Yellow);

        if let Some(stream) = Self::connect_with_retry(&server, retries, delay).await {
            let username = loop {
                xprintln!("Enter your username:" => Color::BrightBlue);
                let mut username = String::new();
                std::io::stdin().read_line(&mut username).map_err(|err| Error::Read(err))?;
                let username = username.trim().to_string();

                if username.len() >= 3 && username.chars().all(|c| c.is_alphanumeric()) {
                    break username;
                } else {
                    xeprintln!("Username must be at least 3 characters long." => Color::Orange);
                }
            };

            xprintln!("Sending username: " => Color::BrightBlue, username => Color::Blue);

            let (mut reader, mut writer) = stream.into_split();

            if let Err(e) = writer.write_all(username.as_bytes()).await {
                xeprintln!("Failed to send username: ", e => Color::Orange);
                return Ok(());
            }

            writer.flush().await.map_err(|_err| Error::Flush)?;

            xprintln!("Welcome, " => Color::BrightGreen, username => Color::Green, "! Type messages to send to other clients." => Color::BrightGreen);

            let receive_task : tokio::task::JoinHandle<Result<(), Error>> = tokio::spawn(async move {
                let mut buffer = [0; 512];
                loop {
                    match reader.read(&mut buffer).await {
                        Ok(0) => {
                            xprintln!("Server closed connection. Exiting..." => Color::BrightRed);
                            std::process::exit(0);
                        }
                        Ok(n) => {
                            let bytes = &buffer[..n];

                            xprintln!("Size: ", format_size(bytes));

                            match Message::from_bytes(bytes) {
                                Ok(response) => {
                                    xprintln!(response.sender, " : ", response.content);
                                }
                                Err(err) => {
                                    break Err(err);
                                }
                            }

                            tokio::io::stdout().flush().await.map_err(|_err| Error::Flush)?;
                        }
                        Err(e) => {
                            break Err(Error::Read(e));
                        }
                    }
                }
            });
            let send_task = tokio::spawn(async move {
                loop {
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
                result = receive_task => {
                    if let Err(err) = result {
                        xeprintln!("Receive task error: ", err => Color::Crimson);
                        return Err(Error::JoinError(err));
                    }
                }
                result = send_task => {
                    if let Err(err) = result {
                        xeprintln!("Send task error: ", err => Color::Crimson);
                        return Err(Error::JoinError(err));
                    }
                }
            }
        } else {
            xeprintln!("Failed to connect to server after", retries => Color::Crimson, "retries.");
        }

        Ok(())
    }

    pub async fn connect_with_retry(server: &Address, retries: u32, delay: u64) -> Option<TcpStream> {
        for attempt in 0..retries {
            xprintln!("Attempt " => Color::BrightYellow, attempt + 1 => Color::BrightYellow,": Connecting to " => Color::Yellow, server => Color::Yellow);

            match TcpStream::connect(server.clone()).await {
                Ok(stream) => {
                    xprintln!("Connected to server successfully " => Color::BrightGreen);
                    return Some(stream);
                }
                Err(err) => {
                    xprintln!("Error Type: " => Color::Red, err.to_string() => Color::Crimson);

                    if attempt < retries - 1 {
                        xprintln!("Retrying in " => Color::Yellow, delay, " seconds..." => Color::Yellow);
                        sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }

        xeprintln!("Failed to connect after " => Color::Crimson, retries, " attempts." => Color::Crimson);
        None
    }
}

fn format_size(bytes: &[u8]) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let size = bytes.len() as f64;

    if size < KB {
        format!("{:.0} B", size)
    } else if size < MB {
        format!("{:.2} KB", size / KB)
    } else if size < GB {
        format!("{:.2} MB", size / MB)
    } else if size < TB {
        format!("{:.2} GB", size / GB)
    } else {
        format!("{:.2} TB", size / TB)
    }
}