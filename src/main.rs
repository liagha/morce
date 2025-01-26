use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::{broadcast, mpsc};
use std::error::Error;
use std::sync::Arc;
use axo_core::{xeprintln, xprintln};

#[derive(Debug, Clone)]
struct Message {
    username: String,
    content: String,
}

impl Message {
    fn serialize(&self) -> String {
        format!("{}|{}", self.username, self.content)
    }

    fn deserialize(input: &str) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
        let parts: Vec<&str> = input.split('|').collect();
        if parts.len() != 2 {
            return Err("Invalid message format".into());
        }

        let username = parts[0].to_string();
        let content = parts[1].to_string();

        Ok(Message { username, content })
    }
}

async fn handle_client(socket: TcpStream, tx: broadcast::Sender<Message>) -> Result<(), Box<dyn Error>> {
    let (reader, writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    let mut username = String::new();
    reader.read_line(&mut username).await?;
    let username = username.trim().to_string();

    let connect_msg = Message {
        username: "Server".to_string(),
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
                    Ok(msg) if msg.username != user => {
                        let serialized = msg.serialize();
                        let mut w = writer.lock().await;
                        let _ = w.write_all(serialized.as_bytes()).await;
                        let _ = w.write_all(b"\n").await;
                        let _ = w.flush().await;
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

    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("192.168.100.13:8080").await?;
    xprintln!("Server listening on port 8080.");

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
                        if let Ok(msg) = Message::deserialize(&buf) {
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
            xprintln!(msg.username, " : ", msg ; Debug);
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

        let serialized = msg.serialize();
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