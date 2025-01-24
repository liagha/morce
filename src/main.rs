use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    id: Uuid,
    username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    from: User,
    to: Option<User>,
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

    let user = User {
        id: Uuid::new_v4(),
        username: username.clone(),
    };

    let connect_msg = Message {
        from: User {
            id: Uuid::nil(),
            username: "Server".to_string(),
        },
        to: None,
        content: format!("{} joined the chat", username),
    };
    tx.send(connect_msg).ok();

    let mut rx = tx.subscribe();
    let writer = Arc::new(tokio::sync::Mutex::new(writer));

    let receive_task = {
        let writer = Arc::clone(&writer);
        let user_id = user.id;
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) if msg.from.id != user_id => {
                        if msg.to.is_none() || msg.to.as_ref().map(|u| u.id) == Some(user_id) {
                            let serialized = serde_json::to_string(&msg).unwrap();
                            let mut w = writer.lock().await;
                            let _ = w.write_all(serialized.as_bytes()).await;
                            let _ = w.write_all(b"\n").await;
                            let _ = w.flush().await;
                        }
                    }
                    Err(_) => break,
                    _ => {}
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
                    let content = buf.trim().to_string();
                    let to = if content.starts_with("@") {
                        let parts: Vec<&str> = content.splitn(2, ' ').collect();
                        let username = parts[0].trim_start_matches('@');
                        Some(User {
                            id: Uuid::nil(), // This should be replaced with the actual user ID lookup
                            username: username.to_string(),
                        })
                    } else {
                        None
                    };

                    let msg = Message {
                        from: user.clone(),
                        to: to.clone(),
                        content: if to.is_some() {
                            content.splitn(2, ' ').nth(1).unwrap_or("").to_string()
                        } else {
                            content
                        },
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

    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("192.168.100.13:8080").await?;
    println!("Server listening on port 8080");

    let (tx, _) = broadcast::channel(100);

    loop {
        let (socket, _) = listener.accept().await?;
        let tx = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, tx).await {
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
            if let Some(to) = msg.to {
                println!("[DM from {}]: {}", msg.from.username, msg.content);
            } else {
                println!("{}: {}", msg.from.username, msg.content);
            }
        }
    });

    let mut stdin = BufReader::new(tokio::io::stdin());
    loop {
        let mut input = String::new();
        stdin.read_line(&mut input).await?;

        let msg = Message {
            from: User {
                id: Uuid::nil(), // This should be replaced with the actual user ID
                username: username.clone(),
            },
            to: if input.starts_with("@") {
                let parts: Vec<&str> = input.splitn(2, ' ').collect();
                let username = parts[0].trim_start_matches('@');
                Some(User {
                    id: Uuid::nil(), // This should be replaced with the actual user ID lookup
                    username: username.to_string(),
                })
            } else {
                None
            },
            content: if input.starts_with("@") {
                input.splitn(2, ' ').nth(1).unwrap_or("").to_string()
            } else {
                input.trim().to_string()
            },
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