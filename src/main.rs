use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use axo_core::{xeprintln, xprint, xprintln, Color};
use tokio::time::sleep;

static SERVER: &str = "0.0.0.0:6000";
static ADDR: &str = "192.168.100.195:6000";

type Sender = mpsc::UnboundedSender<String>;
type Receiver = mpsc::UnboundedReceiver<String>;

struct Client {
    username: String,
    sender: Sender,
}

async fn run_server(address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    let clients = Arc::new(Mutex::new(HashMap::new()));
    xprintln!("Server listening on " => Color::BrightGreen, address => Color::Green);

    loop {
        match listener.accept().await {
            Ok((mut stream, address)) => {
                xprintln!("New connection from " => Color::BrightBlue, address => Color::Blue ; Debug);

                let mut buffer = [0; 512];
                let n = stream.read(&mut buffer).await?;

                if n == 0 {
                    continue;
                }

                let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();

                let clients = Arc::clone(&clients);

                tokio::spawn(async move {
                    handle_client(stream, clients, username).await;
                });
            }

            Err(err) => {
                xeprintln!("Failed to accept connection: ", err => Color::Orange ; Debug);
            }
        }
    }
}

async fn handle_client(stream: TcpStream, clients: Arc<Mutex<HashMap<String, Client>>>, username: String) {
    let (tx, mut rx) = mpsc::unbounded_channel();

    clients.lock().await.insert(username.clone(), Client { username: username.clone(), sender: tx });

    xprintln!("User '" => Color::Magenta, username => Color::Pink, "' joined the chat." => Color::Magenta);

    let (mut reader, mut writer) = stream.into_split();
    let username_clone = username.clone();
    let clients_clone = Arc::clone(&clients);

    let receive_task = tokio::spawn(async move {
        let mut buffer = [0; 512];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    xprintln!("User '" => Color::BrightRed, username_clone => Color::Crimson, "' disconnected." => Color::BrightRed);
                    clients_clone.lock().await.remove(&username_clone);
                    break;
                }
                Ok(n) => {
                    let message = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    let clients = clients_clone.lock().await;

                    if message.starts_with('@') {
                        if let Some((target, msg)) = message.split_once(' ') {
                            let target_username = target.trim_start_matches('@');
                            if let Some(client) = clients.get(target_username) {
                                xprintln!("Private message from ' " => Color::BrightBlue, username_clone => Color::Cyan,
                                    " ' to ' " => Color::BrightBlue, target_username => Color::Cyan, " ': ", msg => Color::Blue);

                                let _ = client.sender.send(format!("(Private) {}: {}", username_clone, msg));
                            } else {
                                println!("User '{}' tried to message '{}', but they are not online.", username_clone, target_username);
                                if let Some(sender) = clients.get(&username_clone) {
                                    let _ = sender.sender.send("User not found".to_string());
                                }
                            }
                            continue;
                        }
                    }

                    xprintln!(username_clone => Color::BrightBlue, " : " => Color::Blue, message);

                    for (_, client) in clients.iter() {
                        if client.username != username_clone {
                            let _ = client.sender.send(format!("{}: {}", username_clone, message));
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let _ = writer.write_all(message.as_bytes()).await;
        }
    });

    tokio::select! {
        _ = receive_task => {}
        _ = send_task => {}
    }
}

async fn run_client(address: &str) -> io::Result<()> {
    let retries = 5;
    let delay = 5;

    xprintln!("Connecting to server..." => Color::Yellow);

    if let Some(stream) = connect_with_retry(address, retries, delay).await {
        xprintln!("Enter your username:" => Color::BrightBlue);
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        let username = username.trim().to_string();
        xprintln!("Sending username: " => Color::BrightBlue, username => Color::Blue);

        let (mut reader, mut writer) = stream.into_split();

        if let Err(e) = writer.write_all(username.as_bytes()).await {
            xeprintln!("Failed to send username: ", e => Color::Orange);
            return Ok(());
        }
        writer.flush().await?;

        xprintln!("Welcome, " => Color::BrightGreen, username => Color::Green, "! Type messages to send to other clients." => Color::BrightGreen);

        let receive_task = tokio::spawn(async move {
            let mut buffer = [0; 512];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        xprintln!("Server disconnected!" => Color::Crimson);
                        break;
                    }
                    Ok(n) => {
                        let response = String::from_utf8_lossy(&buffer[..n]);
                        xprintln!("\n{}", response);
                        xprint!("> ");
                        let _ = io::stdout().flush();
                    }
                    Err(e) => {
                        xeprintln!("Failed to read from server: ", e => Color::Crimson);
                        break;
                    }
                }
            }
        });

        let send_task = tokio::spawn(async move {
            loop {
                xprint!("> ");
                let _ = io::stdout().flush();
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    break;
                }

                if let Err(e) = writer.write_all(input.as_bytes()).await {
                    xeprintln!("Failed to send data: ", e => Color::Crimson);
                    break;
                }
                writer.flush().await?;
            }
            Ok::<_, io::Error>(())
        });

        tokio::select! {
            _ = receive_task => {}
            result = send_task => {
                if let Err(e) = result {
                    xeprintln!("Send task error: ", e => Color::Crimson ; Debug);
                }
            }
        }
    } else {
        xeprintln!("Failed to connect to server after", retries => Color::Crimson, "retries.");
    }

    Ok(())
}

async fn connect_with_retry(addr: &str, retries: u32, delay: u64) -> Option<TcpStream> {
    for attempt in 0..retries {
        xprintln!("Attempt " => Color::BrightYellow, attempt + 1 => Color::BrightYellow,": Connecting to " => Color::Yellow, addr => Color::Yellow);

        match TcpStream::connect(addr).await {
            Ok(stream) => {
                xprintln!("Connected to server at " => Color::BrightGreen, addr => Color::Green);
                return Some(stream);
            }
            Err(err) => {
                xprintln!("Attempt ", attempt + 1, ": Failed to connect to server: " => Color::BrightYellow, err => Color::Orange);

                if attempt < retries - 1 {
                    xprintln!("Retrying in " => Color::Yellow, delay, "seconds..." => Color::Yellow);
                    sleep(Duration::from_secs(delay)).await;
                }
            }
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} [server|client]", args[0]);
        return;
    }

    match args[1].as_str() {
        "server" => {
            let address = if let Some(address) = args.get(3) {
                address
            } else {
                SERVER
            };

            if let Err(e) = run_server(address).await {
                xeprintln!("Server error: ", e => Color::Crimson);
            }
        }
        _ => {
            let address = if let Some(address) = args.get(3) {
                address
            } else {
                ADDR
            };

            if let Err(e) = run_client(address).await {
                xeprintln!("Client error: ", e => Color::Crimson);
            }
        }
    }
}