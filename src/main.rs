use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    username: String,
    content: String,
}

#[derive(Debug, Clone)]
struct User {
    username: String,
    writer: Arc<tokio::sync::Mutex<tokio::net::tcp::OwnedWriteHalf>>,
}

use std::collections::HashMap;
use tokio::sync::Mutex;

async fn handle_client(
    socket: TcpStream,
    tx: broadcast::Sender<Message>,
    users: Arc<Mutex<HashMap<String, User>>>,
) -> Result<(), Box<dyn Error>> {
    let (reader, writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    let mut username = String::new();
    reader.read_line(&mut username).await?;
    let username = username.trim().to_string();

    // Add the user to the shared users list
    let user_writer = Arc::new(tokio::sync::Mutex::new(writer));
    {
        let mut users_lock = users.lock().await;
        users_lock.insert(
            username.clone(),
            User {
                username: username.clone(),
                writer: Arc::clone(&user_writer),
            },
        );
    }

    let connect_msg = Message {
        username: "Server".to_string(),
        content: format!("{} joined the chat", username),
    };
    tx.send(connect_msg).ok();

    let mut rx = tx.subscribe();
    let writer = Arc::clone(&user_writer);

    let receive_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if let Some(target) = msg.content.strip_prefix("@") {
                        // Direct message handling
                        if let Some((target_user, message)) = target.split_once(' ') {
                            let users_lock = users.lock().await;
                            if let Some(user) = users_lock.get(target_user) {
                                let mut target_writer = user.writer.lock().await;
                                let serialized = serde_json::to_string(&msg).unwrap();
                                target_writer.write_all(serialized.as_bytes()).await.unwrap();
                                target_writer.write_all(b"\n").await.unwrap();
                                target_writer.flush().await.unwrap();
                            }
                        }
                    } else {
                        // Broadcast message
                        if msg.username != username {
                            let serialized = serde_json::to_string(&msg).unwrap();
                            let mut w = writer.lock().await;
                            let _ = w.write_all(serialized.as_bytes()).await;
                            let _ = w.write_all(b"\n").await;
                            let _ = w.flush().await;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

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
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::try_join!(receive_task, send_task);

    // Remove the user from the shared users list
    {
        let mut users_lock = users.lock().await;
        users_lock.remove(&username);
    }

    let disconnect_msg = Message {
        username: "Server".to_string(),
        content: format!("{} left the chat", username),
    };
    tx.send(disconnect_msg).ok();

    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("192.168.100.13:8080").await?;
    println!("Server listening on port 8080");

    let (tx, _) = broadcast::channel(100);
    let users = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = listener.accept().await?;
        let tx = tx.clone();
        let users = Arc::clone(&users);

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, tx, users).await {
                eprintln!("Error handling client: {}", e);
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
                            let _ = msg_tx.send(msg).await;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    });

    let _print_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            println!("{}: {}", msg.username, msg.content);
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
        _ => println!("Usage: cargo run -- [server/client] [username]"),
    }

    Ok(())
}