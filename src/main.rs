use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::sync::Arc;
use axo_core::{xeprintln, xprintln, Color};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    username: String,
    content: String,
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

    let connect_msg = Message {
        username: "Server".to_string(),
        content: format!("{} joined the chat", username),
    };

    xprintln!("[ ", username => Color::Green, " ]", " joined the server." => Color::BrightGreen);

    if let Err(err) = tx.send(connect_msg) {
        xeprintln!("Error sending message: {}", err);
    }

    let mut rx = tx.subscribe();
    let writer = Arc::new(tokio::sync::Mutex::new(writer));

    let receive_task = {
        let writer = Arc::clone(&writer);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        let serialized = serde_json::to_string(&msg).unwrap();
                        let mut w = writer.lock().await;
                        if let Err(err) = w.write_all(serialized.as_bytes()).await {
                            xeprintln!("Error writing to client: {}", err);
                        }
                        if let Err(err) = w.write_all(b"\n").await {
                            xeprintln!("Error writing newline to client: {}", err);
                        }
                        if let Err(err) = w.flush().await {
                            xeprintln!("Error flushing to client: {}", err);
                        }
                    }
                    Err(err) => {
                        xeprintln!("Error receiving message: {}", err);
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
                Ok(_) => {
                    let msg = Message {
                        username: username.clone(),
                        content: buf.trim().to_string(),
                    };
                    if tx.send(msg).is_err() {
                        xeprintln!("Error sending message from client.");
                        break;
                    }
                }
                Err(err) => {
                    xeprintln!("Error reading from client: {}", err);
                    break;
                },
            }
        }
    });

    let _ = tokio::try_join!(receive_task, send_task);

    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("192.168.100.13:8080").await?;
    xprintln!("Server listening on port " => Color::BrightGreen, "8080" => Color::Green);

    let (tx, _) = broadcast::channel(100);

    loop {
        let (socket, _) = listener.accept().await?;
        let tx = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, tx).await {
                xeprintln!("Error handling client: {}", e);
            }
        });
    }
}

async fn client(username: String) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("192.168.100.13:8080").await?;
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
                        if let Ok(msg) = serde_json::from_str::<Message>(&buf) {
                            if let Err(err) = msg_tx.send(msg).await {
                                xeprintln!("Error sending message to receiver: {}", err);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    });

    let _print_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            match msg.username.as_str() {
                "Server" => {
                    xprintln!(msg.username => Color::BrightGreen, " => ", msg.content => Color::Green);
                },
                _ => {
                    xprintln!(msg.username, " : ", msg.content);
                }
            };
        }
    });

    let mut stdin = BufReader::new(tokio::io::stdin());

    loop {
        let mut input = String::new();
        stdin.read_line(&mut input).await?;

        let msg = Message {
            username: username.clone(),
            content: input.trim().to_string(),
        };

        let serialized = serde_json::to_string(&msg)?;
        if let Err(err) = writer.write_all(serialized.as_bytes()).await {
            xeprintln!("Error writing message: {}", err);
        }
        if let Err(err) = writer.write_all(b"\n").await {
            xeprintln!("Error writing newline to server: {}", err);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "server".to_string());

    match mode.as_str() {
        "server" => {
            if let Err(e) = server().await {
                xeprintln!("Server error: {}", e);
            }
        }
        "client" => {
            let username = std::env::args().nth(2)
                .unwrap_or_else(|| "Anonymous".to_string());
            if let Err(e) = client(username).await {
                xeprintln!("Client error: {}", e);
            }
        }
        _ => xprintln!("Usage: cargo run -- [server/client] [username]"),
    }

    Ok(())
}