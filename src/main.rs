use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;
use axo_core::{xeprintln, xprintln};

// Enhanced error handling with detailed logging
#[derive(Debug)]
enum ChatError {
    ConnectionError(String),
    SerializationError(String),
    DeserializationError(String),
    IOError(String),
    ChannelError(String),
}

impl Display for ChatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ChatError::ConnectionError(msg) => write!(f, "Connection Error: {}", msg),
            ChatError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            ChatError::DeserializationError(msg) => write!(f, "Deserialization Error: {}", msg),
            ChatError::IOError(msg) => write!(f, "IO Error: {}", msg),
            ChatError::ChannelError(msg) => write!(f, "Channel Error: {}", msg),
        }
    }
}

impl Error for ChatError {}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
struct User {
    name: String,
}

impl User {
    pub fn from_str(name: &str) -> Self {
        Self {
            name: name.to_string()
        }
    }

    pub fn from_string(name: String) -> Self {
        Self {
            name
        }
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    from: User,
    to: Option<User>,
    content: String,
    timestamp: String,
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.timestamp, self.from, self.content)
    }
}

// Robust logging and error tracking function
fn log_error(error: &dyn Error) {
    xeprintln!("Detailed Error Log:");
    xeprintln!("Error Type: {}", error.to_string());

    // Optional: Log stack trace if available
    if let Some(source) = error.source() {
        xeprintln!("Error Source: {}", source);
    }
}

async fn handle_client(
    socket: TcpStream,
    tx: broadcast::Sender<Message>
) -> Result<(), ChatError> {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    // Robust username reading with timeout and error handling
    let mut username = String::new();
    match tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut username)).await {
        Ok(Ok(_)) => {
            let username_str = username.trim().to_string();
            if username_str.is_empty() {
                return Err(ChatError::ConnectionError("Empty username".to_string()));
            }

            xprintln!(username_str.clone(), " connected successfully.");

            // Enhanced connection message with timestamp
            let connect_msg = Message {
                from: User::from_str("Server"),
                to: None,
                content: format!("{} joined the chat", username_str),
                timestamp: chrono::Local::now().to_rfc3339(),
            };

            // Robust channel sending with error handling
            tx.send(connect_msg)
                .map_err(|e| ChatError::ChannelError(e.to_string()))?;

            let mut rx = tx.subscribe();
            let writer = Arc::new(tokio::sync::Mutex::new(writer));

            let receive_task = {
                let writer = Arc::clone(&writer);
                let username_for_task = username_str.clone();

                tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(msg) => {
                                // Comprehensive serialization with error handling
                                match serde_json::to_string(&msg) {
                                    Ok(serialized) => {
                                        let mut w = writer.lock().await;
                                        if let Err(e) = w.write_all(serialized.as_bytes()).await {
                                            xeprintln!("Write error for {}: {}", username_for_task, e);
                                            break;
                                        }
                                        if let Err(e) = w.write_all(b"\n").await {
                                            xeprintln!("Newline write error for {}: {}", username_for_task, e);
                                            break;
                                        }
                                        if let Err(e) = w.flush().await {
                                            xeprintln!("Flush error for {}: {}", username_for_task, e);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        xeprintln!("Serialization error for {}: {}", username_for_task, e);
                                        break;
                                    }
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                                xeprintln!("Message queue lagged, skipped {} messages for {}", skipped, username_for_task);
                            }
                            Err(err) => {
                                xeprintln!("Receive error for {}: {}", username_for_task, err);
                                break;
                            }
                        }
                    }
                })
            };

            let send_task = {
                let username_for_task = username_str.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(reader);
                    let mut buf = String::new();
                    loop {
                        buf.clear();
                        match reader.read_line(&mut buf).await {
                            Ok(0) => {
                                xprintln!("{} disconnected gracefully", username_for_task);
                                break;
                            }
                            Ok(_) => {
                                let trimmed = buf.trim();
                                if trimmed.is_empty() {
                                    continue;
                                }

                                let msg = Message {
                                    from: User::from_string(username_for_task.clone()),
                                    to: None,
                                    content: trimmed.to_string(),
                                    timestamp: chrono::Local::now().to_rfc3339(),
                                };

                                xprintln!(username_for_task.clone(), " said ", msg);

                                if let Err(err) = tx.send(msg) {
                                    xeprintln!("Failed to broadcast message for {}: {}", username_for_task, err);
                                    break;
                                }
                            }
                            Err(e) => {
                                xeprintln!("Read error for {}: {}", username_for_task, e);
                                break;
                            },
                        }
                    }
                })
            };

            // Comprehensive error handling for task joining
            match tokio::try_join!(receive_task, send_task) {
                Ok(_) => Ok(()),
                Err(e) => {
                    xeprintln!("Task join error for {}: {}", username_str, e);
                    Err(ChatError::IOError(e.to_string()))
                }
            }
        }
        Ok(Err(e)) => {
            xeprintln!("Username read error: {}", e);
            Err(ChatError::IOError(e.to_string()))
        }
        Err(_) => {
            xeprintln!("Username read timeout");
            Err(ChatError::ConnectionError("Username read timeout".to_string()))
        }
    }
}

async fn server() -> Result<(), ChatError> {
    // Robust binding with multiple retry mechanism
    let listener = match TcpListener::bind("0.0.0.0:8080").await {
        Ok(listener) => listener,
        Err(e) => {
            xeprintln!("Server binding error: {}", e);
            return Err(ChatError::ConnectionError(e.to_string()));
        }
    };

    xprintln!("Server listening on 0.0.0.0:8080");

    // Larger channel buffer to handle more concurrent messages
    let (tx, _) = broadcast::channel(10000);

    loop {
        let (socket, addr) = match listener.accept().await {
            Ok(result) => result,
            Err(e) => {
                xeprintln!("Connection accept error: {}", e);
                continue; // Continue listening instead of stopping entire server
            }
        };

        xprintln!("New connection from: {}", addr);

        let tx = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, tx).await {
                log_error(&e);
            }
        });
    }
}

async fn client(username: String) -> Result<(), ChatError> {
    // Robust connection with retry mechanism
    let mut stream = match TcpStream::connect("192.168.100.13:8080").await {
        Ok(stream) => stream,
        Err(e) => {
            xeprintln!("Connection failed: {}", e);
            return Err(ChatError::ConnectionError(e.to_string()));
        }
    };

    // Robust username sending
    stream.write_all(format!("{}\n", username).as_bytes())
        .await
        .map_err(|e| ChatError::IOError(e.to_string()))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(1000);

    let _read_task = tokio::spawn(async move {
        let mut buf = String::new();
        loop {
            buf.clear();
            match reader.read_line(&mut buf).await {
                Ok(0) => {
                    xeprintln!("Connection closed by server.");
                    break;
                }
                Ok(_) => {
                    match serde_json::from_str::<Message>(&buf) {
                        Ok(msg) => {
                            xprintln!("Received message: ", msg);
                            if msg_tx.send(msg).await.is_err() {
                                xeprintln!("Failed to send message to print task");
                                break;
                            }
                        }
                        Err(e) => {
                            xeprintln!("Failed to deserialize message: ", e);
                        }
                    }
                }
                Err(e) => {
                    xeprintln!("Error reading from socket: ", e);
                    break;
                }
            }
        }
    });

    let _print_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            xprintln!("Received message: ", msg);
        }
    });

    let mut stdin = BufReader::new(tokio::io::stdin());
    loop {
        let mut input = String::new();
        match stdin.read_line(&mut input).await {
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let msg = Message {
                    from: User::from_string(username.clone()),
                    to: None,
                    content: trimmed.to_string(),
                    timestamp: chrono::Local::now().to_rfc3339(),
                };

                // Robust message sending
                match serde_json::to_string(&msg) {
                    Ok(serialized) => {
                        if let Err(e) = writer.write_all(serialized.as_bytes()).await {
                            xeprintln!("Failed to write message: {}", e);
                            break;
                        }
                        if let Err(e) = writer.write_all(b"\n").await {
                            xeprintln!("Failed to write newline: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        xeprintln!("Serialization error: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                xeprintln!("Input reading error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Enhanced error handling for mode selection
    let mode = std::env::args().nth(1).unwrap_or_else(|| "server".to_string());

    match mode.as_str() {
        "server" => {
            match server().await {
                Ok(_) => Ok(()),
                Err(e) => {
                    log_error(&e);
                    Err(Box::new(e))
                }
            }
        }
        "client" => {
            let username = std::env::args().nth(2)
                .unwrap_or_else(|| "Anonymous".to_string());

            match client(username).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    log_error(&e);
                    Err(Box::new(e))
                }
            }
        }
        _ => {
            xprintln!("Usage: cargo run -- [server/client] [username]");
            Err(Box::new(ChatError::IOError("Invalid mode".to_string())))
        }
    }?;

    Ok(())
}