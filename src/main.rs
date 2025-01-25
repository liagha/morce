use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fmt::Formatter;
use std::sync::Arc;
use std::time::Duration;
use axo_core::{xeprintln, xprintln};

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

impl core::fmt::Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum MessageType {
    Connect,
    Disconnect,
    Chat,
    ServerMessage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    from: User,
    to: Option<User>,
    content: String,
    msg_type: MessageType,
}

impl core::fmt::Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self.content)
    }
}

async fn handle_client(
    socket: TcpStream,
    tx: broadcast::Sender<Message>
) -> Result<(), Box<dyn Error>> {
    let (reader, writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    let mut username = String::new();

    reader.read_line(&mut username).await?;
    let username = username.trim().to_string();

    if username.is_empty() {
        xeprintln!("Empty username received.");
        return Err("Empty username".into());
    }

    xprintln!("Client ", username, " attempting to connect");

    let connect_msg = Message {
        from: User::from_str("Server"),
        to: None,
        content: format!("{} joined the chat", username),
        msg_type: MessageType::Connect,
    };

    if let Err(e) = tx.send(connect_msg) {
        xeprintln!("Failed to broadcast connection message: ", e);
    }

    let mut rx = tx.subscribe();
    let writer = Arc::new(tokio::sync::Mutex::new(writer));

    {
        let mut w = writer.lock().await;
        let confirm_msg = Message {
            from: User::from_str("Server"),
            to: Some(User::from_string(username.clone())),
            content: "Connected successfully".to_string(),
            msg_type: MessageType::ServerMessage,
        };
        let serialized = serde_json::to_string(&confirm_msg)?;
        w.write_all(serialized.as_bytes()).await?;
        w.write_all(b"\n").await?;
        w.flush().await?;
    }

    let receive_task = {
        let writer = Arc::clone(&writer);
        let _username = username.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if msg.from.name != _username || matches!(msg.msg_type, MessageType::ServerMessage) {
                            let serialized = match serde_json::to_string(&msg) {
                                Ok(s) => s,
                                Err(e) => {
                                    xeprintln!("Serialization error: ", e);
                                    continue;
                                }
                            };

                            let mut w = writer.lock().await;
                            if let Err(e) = w.write_all(serialized.as_bytes()).await {
                                xeprintln!("Write error: ", e);
                                break;
                            }
                            if let Err(e) = w.write_all(b"\n").await {
                                xeprintln!("Newline write error: ", e);
                                break;
                            }
                            if let Err(e) = w.flush().await {
                                xeprintln!("Flush error: ", e);
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        xprintln!("Message channel lagged, skipped ", skipped, " messages");
                    }
                    Err(_) => break,
                }
            }
        })
    };

    let send_task = tokio::spawn({
        let tx_clone = tx.clone();
        async move {
            let mut buf = String::new();
            loop {
                buf.clear();
                match reader.read_line(&mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let msg = Message {
                            from: User::from_string(username.clone()),
                            to: None,
                            content: buf.trim().to_string(),
                            msg_type: MessageType::Chat,
                        };

                        let content = msg.content.trim();
                        xprintln!(username, " sent: ", content);

                        if tx_clone.send(msg).is_err() {
                            xeprintln!("Failed to send message to broadcast channel.");
                            break;
                        }
                    }
                    Err(e) => {
                        xeprintln!("Read error: ", e);
                        break;
                    }
                }
            }

            let disconnect_msg = Message {
                from: User::from_str("Server"),
                to: None,
                content: format!("{} left the chat", username),
                msg_type: MessageType::Disconnect,
            };

            let _ = tx_clone.send(disconnect_msg);
        }
    });

    let _ = tokio::try_join!(receive_task, send_task);

    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("192.168.100.13:8080").await?;
    xprintln!("Server listening on port 8080");

    let (tx, _) = broadcast::channel(100);

    loop {
        let (socket, addr) = listener.accept().await?;
        xprintln!("New connection from: ", addr);

        let tx = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, tx).await {
                xeprintln!("Error handling client: ", e);
            }
        });
    }
}

async fn client(username: String) -> Result<(), Box<dyn Error>> {
    xprintln!("Attempting to connect as ", username);

    let mut retry_count = 0;

    let max_retries = 3;
    let mut stream = loop {
        match TcpStream::connect("192.168.100.13:8080").await {
            Ok(stream) => break stream,
            Err(e) => {
                retry_count += 1;
                if retry_count > max_retries {
                    return Err(format!("Failed to connect after {} attempts: {}", max_retries, e).into());
                }
                xeprintln!("Connection attempt ", retry_count, " failed: ", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    };

    stream.write_all(format!("{}\n", username).as_bytes()).await?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let (msg_tx, mut msg_rx) = mpsc::channel(100);

    let _read_task = tokio::spawn({
        let msg_tx = msg_tx.clone();
        async move {
            let mut buf = String::new();
            loop {
                buf.clear();
                match reader.read_line(&mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        match serde_json::from_str::<Message>(&buf) {
                            Ok(msg) => {
                                if msg.msg_type == MessageType::ServerMessage {
                                    xprintln!("Server: ", msg.content);
                                }

                                if let Err(e) = msg_tx.try_send(msg) {
                                    xeprintln!("Failed to send message: ", e);
                                }
                            }
                            Err(e) => xeprintln!("Failed to parse message: ", e),
                        }
                    }
                    Err(e) => {
                        xeprintln!("Read error: {}", e);
                        break;
                    }
                }
            }
        }
    });

    let _print_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            match msg.msg_type {
                MessageType::Chat => {
                    println!("{}: {}", msg.from.name, msg.content);
                }
                MessageType::Connect | MessageType::Disconnect => {
                    println!("*** {}", msg.content);
                }
                MessageType::ServerMessage => {
                    println!("Server: {}", msg.content);
                }
            }
        }
    });

    let mut stdin = BufReader::new(tokio::io::stdin());
    loop {
        let mut input = String::new();
        stdin.read_line(&mut input).await?;

        let msg = Message {
            from: User::from_string(username.clone()),
            to: None,
            content: input.trim().to_string(),
            msg_type: MessageType::Chat,
        };

        let serialized = serde_json::to_string(&msg)?;
        writer.write_all(serialized.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "server".to_string());

    match mode.as_str() {
        "server" => server().await?,
        "client" => {
            let username = std::env::args().nth(2)
                .unwrap_or_else(|| "Anonymous".to_string());
            client(username).await?
        }
        _ => println!("Usage: cargo run -- [server/client] [username]"),
    }

    Ok(())
}