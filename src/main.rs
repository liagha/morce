use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc, Mutex};
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct User {
    username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    from: User,
    to: Option<User>, // None means broadcast to all
    content: String,
}

#[derive(Debug)]
struct UserConnection {
    user: User,
    writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
}

#[derive(Debug)]
struct UserManager {
    users: Arc<Mutex<HashMap<String, UserConnection>>>,
}

impl UserManager {
    fn new() -> Self {
        UserManager {
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn add_user(&self, user: User, writer: tokio::net::tcp::OwnedWriteHalf) {
        let user_conn = UserConnection {
            user: user.clone(),
            writer: Arc::new(Mutex::new(writer)),
        };
        self.users.lock().await.insert(user.username.clone(), user_conn);
    }

    async fn remove_user(&self, username: &str) {
        self.users.lock().await.remove(username);
    }

    async fn send_message(&self, message: Message) -> Result<(), Box<dyn Error>> {
        let users = self.users.lock().await;
        match message.to.clone() {
            Some(to_user) => {
                if let Some(user_conn) = users.get(&to_user.username) {
                    let serialized = serde_json::to_string(&message)?;
                    let mut writer = user_conn.writer.lock().await;
                    writer.write_all(serialized.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                }
            }
            None => {
                for user_conn in users.values() {
                    let serialized = serde_json::to_string(&message)?;
                    let mut writer = user_conn.writer.lock().await;
                    writer.write_all(serialized.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                }
            }
        }
        Ok(())
    }
}

async fn handle_client(
    socket: TcpStream,
    user_manager: Arc<UserManager>,
    tx: broadcast::Sender<Message>
) -> Result<(), Box<dyn Error>> {
    let (reader, writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    let mut username = String::new();
    reader.read_line(&mut username).await?;
    let username = username.trim().to_string();

    let user = User { username: username.clone() };
    user_manager.add_user(user.clone(), writer).await;

    let connect_msg = Message {
        from: User { username: "Server".to_string() },
        to: None,
        content: format!("{} joined the chat", username),
    };
    tx.send(connect_msg).ok();

    let mut rx = tx.subscribe();

    let receive_task = {
        let user_manager = Arc::clone(&user_manager);
        let _username = username.clone();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if let Err(e) = user_manager.send_message(msg).await {
                            eprintln!("Error sending message: {}", e);
                        }
                    }
                    Err(_) => break,
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
                        from: user.clone(),
                        to: None, // Default to broadcast
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

    user_manager.remove_user(&username).await;

    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("192.168.100.13:8080").await?;
    println!("Server listening on port 8080");

    let (tx, _) = broadcast::channel(100);
    let user_manager = Arc::new(UserManager::new());

    loop {
        let (socket, _) = listener.accept().await?;
        let tx = tx.clone();
        let user_manager = Arc::clone(&user_manager);

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, user_manager, tx).await {
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
            println!("{}: {}", msg.from.username, msg.content);
        }
    });

    let mut stdin = BufReader::new(tokio::io::stdin());
    loop {
        let mut input = String::new();
        stdin.read_line(&mut input).await?;

        let msg = Message {
            from: User { username: username.clone() },
            to: None, // Default to broadcast
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