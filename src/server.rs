use std::{
    sync::Arc,
    time::Duration,
    collections::HashSet,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time::sleep,
    io::{AsyncWriteExt, AsyncReadExt},
};
use tokio::time::timeout;
use crate::{client::Client, errors::Error, message::{Message, Content}, Sender};
use axo_core::{xeprintln, xprintln, Color};

pub struct Server {
    pub listener: TcpListener,
    pub clients: Arc<Mutex<HashSet<Client>>>,
}

impl Server {
    pub async fn start(address: &str) -> Result<Self, Error> {
        xprintln!("Attempting to bind to address: " => Color::Yellow, address => Color::Yellow);

        let listener = TcpListener::bind(address).await.map_err(|e| Error::ServerBindFailed(e))?;
        xprintln!("Server listening on " => Color::BrightGreen, address => Color::Green);

        Ok(Self {
            listener,
            clients: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub async fn run(&self) -> Result<(), Error> {
        loop {
            tokio::select! {
                Ok((stream, addr)) = self.listener.accept() => {
                    xprintln!("New connection from " => Color::BrightBlue, addr => Color::Blue ; Debug);
                    self.handle_new_connection(stream).await?;
                }
                _ = tokio::signal::ctrl_c() => {
                    xprintln!("Shutting down server..." => Color::BrightRed);
                    self.shutdown().await?;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_new_connection(&self, mut stream: TcpStream) -> Result<(), Error> {
        let mut buffer = [0; crate::BUFFER_SIZE];
        let n = stream.read(&mut buffer).await.map_err(|e| Error::MessageReceiveFailed(e))?;

        if n == 0 {
            return Ok(())
        }

        let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
        let clients = Arc::clone(&self.clients);

        tokio::spawn(async move {
            return Self::handle_client(clients, username, stream).await;
        });

        Ok(())
    }

    async fn handle_client(clients: Arc<Mutex<HashSet<Client>>>, username: String, stream: TcpStream) -> Result<(), Error> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let client = Client { username: username.clone(), sender: Sender::from(tx) };

        let mut client_list = clients.lock().await;
        if client_list.contains(&client) {
            xeprintln!("Username '", username, "' is already in use." => Color::Crimson);
            return Err(Error::UsernameTaken);
        }
        client_list.insert(client.clone());
        drop(client_list);

        xprintln!("User '" => Color::Magenta, username => Color::Pink, "' joined the chat." => Color::Magenta);

        let (reader, writer) = stream.into_split();
        let username_for_send = username.clone();
        let clients_for_send = Arc::clone(&clients);

        let receive_task = tokio::spawn(Self::receive_messages(clients, username.clone(), reader));
        let send_task = tokio::spawn(Self::send_messages(clients_for_send.clone(), username_for_send, writer, rx));

        match tokio::select! {
            result = receive_task => result,
            result = send_task => result,
        } {
            Ok(inner_result) => {
                match inner_result {
                    Ok(_) => Ok(()),
                    Err(error) => {
                        xprintln!("Client '" => Color::Crimson, username, "' disconnected: " => Color::Crimson, error => Color::Yellow);
                        Self::remove_user(&clients_for_send, &username).await;
                        Err(error)
                    }
                }
            }
            Err(join_error) => {
                xeprintln!("Task join error for '", username, "': ", join_error => Color::Crimson);
                Self::remove_user(&clients_for_send, &username).await;
                Err(Error::TaskJoinFailed(join_error))
            }
        }
    }

    async fn receive_messages(clients: Arc<Mutex<HashSet<Client>>>, username: String, mut reader: tokio::net::tcp::OwnedReadHalf) -> Result<(), Error> {
        let mut length_buffer = [0; 8];
        let mut last_activity = tokio::time::Instant::now();
        const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);

        loop {
            let read_result = timeout(HEARTBEAT_TIMEOUT, reader.read_exact(&mut length_buffer)).await;

            match read_result {
                Ok(Ok(_)) => {
                    last_activity = tokio::time::Instant::now();
                    let message_length = u64::from_be_bytes(length_buffer);
                    let mut message_buffer = vec![0; message_length as usize];
                    let mut bytes_read = 0;

                    while bytes_read < message_length as usize {
                        let remaining = message_length as usize - bytes_read;
                        let chunk_size = std::cmp::min(remaining, crate::BUFFER_SIZE);

                        match reader.read(&mut message_buffer[bytes_read..(bytes_read + chunk_size)]).await {
                            Ok(n) if n > 0 => {
                                bytes_read += n;
                                last_activity = tokio::time::Instant::now();
                            }
                            Ok(_) => continue,
                            Err(e) => {
                                xeprintln!("Error reading message from '", username, "': ", e => Color::Crimson);
                                Self::remove_user(&clients, &username).await;
                                return Err(Error::MessageReceiveFailed(e));
                            }
                        }
                    }

                    match Message::from_bytes(&message_buffer) {
                        Ok(message) => {
                            match message.content {
                                Content::Text(ref text) => {
                                    xprintln!(message.clone());
                                    Self::broadcast_message(&clients, &username, text).await?;
                                }
                                Content::File(file_data) => {
                                    xprintln!(username => Color::BrightBlue, " sent a file: ", file_data.name => Color::Blue);
                                    Self::broadcast_file(&clients, &username, file_data).await?;
                                }
                                Content::Signal(_) => {
                                    xprintln!(message.clone());
                                }
                            }
                        }
                        Err(e) => {
                            xeprintln!("Error parsing message from '", username, "': ", e => Color::Crimson);
                            Self::remove_user(&clients, &username).await;
                            return Err(e);
                        }
                    }
                }
                Ok(Err(e)) => {
                    Self::remove_user(&clients, &username).await;
                    return Err(Error::MessageReceiveFailed(e));
                }
                Err(_) => {
                    if last_activity.elapsed() > HEARTBEAT_TIMEOUT {
                        xeprintln!("No heartbeat from '", username, "' - disconnecting" => Color::Yellow);
                        Self::remove_user(&clients, &username).await;
                        return Err(Error::ClientDisconnected(username, Error::HeartBeatTimeOut.into()));
                    }
                }
            }
        }
    }

    async fn send_messages(clients: Arc<Mutex<HashSet<Client>>>, username: String, mut writer: tokio::net::tcp::OwnedWriteHalf, mut rx: tokio::sync::mpsc::UnboundedReceiver<Message>) -> Result<(), Error> {
        while let Some(message) = rx.recv().await {
            let message_bytes = message.as_bytes()?;
            let message_len = message_bytes.len() as u64;

            if let Err(e) = writer.write_all(&message_len.to_be_bytes()).await {
                Self::remove_user(&clients, &username).await;
                return Err(Error::BytesWriteFailed(e));
            }

            for chunk in message_bytes.chunks(crate::BUFFER_SIZE) {
                if let Err(e) = writer.write_all(chunk).await {
                    Self::remove_user(&clients, &username).await;
                    return Err(Error::BytesWriteFailed(e));
                }
            }

            if let Err(e) = writer.flush().await {
                Self::remove_user(&clients, &username).await;
                return Err(Error::StreamFlushFailed(e));
            }
        }

        Ok(())
    }

    async fn broadcast_message(clients: &Arc<Mutex<HashSet<Client>>>, sender_username: &str, text: &str) -> Result<(), Error> {
        let message = Message::from(text, &sender_username.to_string());
        let clients = clients.lock().await;

        for client in clients.iter() {
            if client.username != sender_username {
                client.sender.send(message.clone()).map_err(|e| Error::MessageSendFailed(e))?;
            }
        }

        Ok(())
    }

    async fn broadcast_file(clients: &Arc<Mutex<HashSet<Client>>>, sender_username: &str, file_data: crate::message::FileData) -> Result<(), Error> {
        let message = Message::from_file(file_data.data.clone(), file_data.name.clone(), &sender_username.to_string());
        let clients = clients.lock().await;

        for client in clients.iter() {
            if client.username != sender_username {
                client.sender.send(message.clone()).map_err(|e| Error::MessageSendFailed(e))?;
            }
        }

        Ok(())
    }

    async fn remove_user(clients: &Arc<Mutex<HashSet<Client>>>, username: &str) {
        let mut clients = clients.lock().await;
        clients.remove(&Client { username: username.to_string(), sender: Sender::new() });

        let disconnect_message = Message::from(&format!("{} has left the chat.", username), &"Server".to_string());

        for client in clients.iter() {
            if client.username != username {
                client.sender.send(disconnect_message.clone()).map_err(|e| Error::MessageSendFailed(e)).ok();
            }
        }
    }

    async fn shutdown(&self) -> Result<(), Error> {
        let mut clients = self.clients.lock().await;

        for client in clients.iter() {
            let message = Message::from("Server is shutting down...", &"Server".to_string());
            client.sender.send(message).map_err(|e| Error::MessageSendFailed(e))?;
        }

        sleep(Duration::from_secs(1)).await;
        clients.clear();

        Ok(())
    }
}