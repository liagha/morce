use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fmt::Formatter;
use std::sync::Arc;
use axo_core::{xeprintln, xprintln, Color};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    from: User,
    to: Option<User>,
    content: String,
}

impl core::fmt::Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let deserialized: Message =
            if let Ok(msg) = serde_json::from_str(&self.content.to_string()) {
                msg
            } else {
                self.clone()
            };

        let content = deserialized.content;

        write!(f, "{}", content)
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

    xprintln!(username, " connected.");

    let connect_msg = Message {
        from: User::from_str("Server"),
        to: None,
        content: format!("{} joined the chat", username),
    };

    tx.send(connect_msg).ok();

    let mut rx = tx.subscribe();
    let writer = Arc::new(tokio::sync::Mutex::new(writer));

    let receive_task = {
        let writer = Arc::clone(&writer);
        let user = username.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        let serialized = serde_json::to_string(&msg).unwrap();
                        let mut w = writer.lock().await;
                        let _ = w.write_all(serialized.as_bytes()).await;
                        let _ = w.write_all(b"\n").await;
                        let _ = w.flush().await;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        xprintln!("Lagged");
                    }
                    Err(err) => {
                        xeprintln!("Test Error 1: ", err ; Debug);
                        break
                    }
                }
            }
        })
    };

    let send_task = tokio::spawn(async move {
        let mut buf = String::new();
        loop {
            buf.clear();
            match reader.read_line(&mut buf).await {
                Ok(0) => break,
                Ok(ok) => {
                    xprintln!("Test 3: ", ok);

                    let msg = Message {
                        from: User::from_string(username.clone()),
                        to: None,
                        content: buf.trim().to_string(),
                    };

                    xprintln!(username.clone(), " said ", msg);

                    if let Err(err) = tx.send(msg) {
                        xeprintln!("Test Error 4: ", err ; Debug);
                        break;
                    }
                }
                Err(err) => {
                    xeprintln!("Test Error 2: ", err ; Debug);
                    break
                },
            }
        }
    });

    let _ = tokio::try_join!(receive_task, send_task);

    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("192.168.100.13:8080").await?;
    xprintln!("Server listening on port 8080");

    let (tx, _) = broadcast::channel(1000);

    loop {
        let (socket, _) = listener.accept().await?;
        let tx = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, tx).await {
                xeprintln!("Error handling client: ", e);
            }
        });
    }
}

async fn client(username: String) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("192.168.100.13:8080").await?;

    stream.write_all(format!("{}\n", username).as_bytes()).await?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(100);

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
                            xprintln!("Received message: ", msg.from, " : ", msg.content);
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
            xprintln!("Received message: ", msg.from.name, " : ", msg.content);
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
        };

        let serialized = serde_json::to_string(&msg)?;
        writer.write_all(serialized.as_bytes()).await?;
        writer.write_all(b"\n").await?;
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
        _ => xprintln!("Usage: cargo run -- [server/client] [username]"),
    }

    Ok(())
}