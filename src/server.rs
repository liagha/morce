use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time::sleep,
    io::{AsyncWriteExt, AsyncReadExt},
};
use crate::{
    client::Client,
    errors::Error,
    message::{Message, MessageType, Content},
};
use axo_core::{xeprintln, xprintln, Color};

pub struct Server {
    pub listener: TcpListener,
    pub clients: Arc<Mutex<HashMap<String, Client>>>,
}

impl Server {
    pub async fn start(address: &str) -> Result<Self, Error> {
        xprintln!("Attempting to bind to address: " => Color::Yellow, address => Color::Yellow);

        let listener = match TcpListener::bind(address).await {
            Ok(listener) => listener,
            Err(e) => {
                return Err(Error::ServerBindFailed(e));
            }
        };

        xprintln!("Server listening on " => Color::BrightGreen, address => Color::Green);

        Ok(Self {
            listener,
            clients: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn run(&self) -> Result<(), Error> {
        loop {
            tokio::select! {
                Ok((mut stream, addr)) = self.listener.accept() => {
                    xprintln!("New connection from " => Color::BrightBlue, addr => Color::Blue ; Debug);

                    let mut buffer = [0; 8192];
                    let n = stream.read(&mut buffer).await.map_err(|e| Error::MessageReceiveFailed(e))?;

                    if n == 0 {
                        continue;
                    }

                    let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    let clients = Arc::clone(&self.clients);

                    tokio::spawn(async move {
                        return Self::handle_client(clients, username, stream).await;
                    });
                }
                _ = tokio::signal::ctrl_c() => {
                    xprintln!("Shutting down server..." => Color::BrightRed);

                    let mut clients = self.clients.lock().await;

                    for (_, client) in clients.iter() {
                        let message = Message::from("Server is shutting down...", "Server".to_string(), MessageType::Public);

                        client.sender.send(message).map_err(|e| Error::MessageSendFailed(e))?;
                    }

                    sleep(Duration::from_secs(1)).await;

                    clients.clear();

                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn handle_client(clients: Arc<Mutex<HashMap<String, Client>>>, username: String, stream: TcpStream) -> Result<(), Error> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        clients.lock().await.insert(username.clone(), Client { username: username.clone(), sender: tx });

        xprintln!("User '" => Color::Magenta, username => Color::Pink, "' joined the chat." => Color::Magenta);

        let (mut reader, mut writer) = stream.into_split();

        let username_for_send = username.clone();
        let clients_for_send = Arc::clone(&clients);

        let receive_task : tokio::task::JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            let mut buffer = [0; 8192];
            loop {
                let mut length_buffer = [0; 8];
                reader.read_exact(&mut length_buffer).await.map_err(|e| Error::MessageReceiveFailed(e))?;
                let length = usize::from_be_bytes(length_buffer);
                xprintln!("Raw length bytes: ", length_buffer ; Debug);  // Print raw bytes
                xprintln!("Test Length: ", length);
                let mut message_bytes = Vec::with_capacity(length);

                while message_bytes.len() < length {
                    let remaining = length - message_bytes.len();
                    let chunk_size = std::cmp::min(remaining, buffer.len());
                    let n = reader.read(&mut buffer[..chunk_size]).await.map_err(|e| {
                        Error::MessageReceiveFailed(e)
                    })?;
                    message_bytes.extend_from_slice(&buffer[..n]);
                }

                let message = Message::from_bytes(&message_bytes)?;

                xprintln!("Message Length: ", length);

                match message.content {
                    Content::Text(ref text) => {
                        if text.starts_with('@') {
                            if let Some((target, msg)) = text.split_once(' ') {
                                let target_username = target.trim_start_matches('@');

                                if let Some(client) = clients.lock().await.get(target_username) {
                                    xprintln!("Private message from " => Color::BrightBlue, "'", username, "'", " to " => Color::BrightBlue, "'", target_username, "'", " : ", msg => Color::Blue);

                                    let message = Message::from(msg, username.clone(), MessageType::Private);
                                    client.sender.send(message).map_err(|e| Error::MessageSendFailed(e))?;
                                } else {
                                    xprintln!("User '" => Color::Red, username, "' tried to message '" => Color::Red, target_username, "', but they are not online." => Color::Red);

                                    if let Some(sender) = clients.lock().await.get(&username) {
                                        let message = Message::from("User not found", "Server".to_string(), MessageType::Private);
                                        sender.sender.send(message).map_err(|e| Error::MessageSendFailed(e))?;
                                    }
                                }
                                continue;
                            } else {
                                return Err(Error::MessageConversionFailed);
                            }
                        }

                        xprintln!(message.clone());

                        for (_, client) in clients.lock().await.iter() {
                            if client.username != username {
                                let message = Message::from(text.as_str(), username.clone(), MessageType::Public);
                                client.sender.send(message).map_err(|e| Error::MessageSendFailed(e))?;
                            }
                        }
                    }
                    Content::File(file_data) => {
                        xprintln!(username => Color::BrightBlue, " sent a file: ", file_data.name => Color::Blue);

                        for (_, client) in clients.lock().await.iter() {
                            if client.username != username {
                                let message = Message::from_file(file_data.data.clone(), file_data.name.clone(), username.clone(), MessageType::Public);
                                client.sender.send(message).map_err(|e| Error::MessageSendFailed(e))?;
                            }
                        }
                    }
                }
            }
        });

        let send_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = writer.write_all(&message.as_bytes()?).await {
                    let mut clients = clients_for_send.lock().await;
                    clients.remove(&username_for_send);
                    return Err(Error::BytesWriteFailed(e));
                }
            }
            Ok(())
        });

        tokio::select! {
        result = receive_task => {
            if let Err(e) = result {
                xeprintln!("Receive task error: ", e => Color::Crimson);
                    return Err(Error::TaskJoinFailed(e));
            }
        }
        result = send_task => {
            if let Err(e) = result {
                xeprintln!("Send task error: ", e => Color::Crimson);
                    return Err(Error::TaskJoinFailed(e));
            }
        }
        }

        Ok(())
    }
}