use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

static SERVER: &str = "0.0.0.0:6000";
static ADDR: &str = "192.168.100.195:6000";

type Sender = mpsc::UnboundedSender<String>;
type Receiver = mpsc::UnboundedReceiver<String>;

struct Client {
    username: String,
    sender: Sender,
}

async fn handle_client(mut stream: TcpStream, clients: Arc<Mutex<HashMap<String, Client>>>, username: String) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    clients.lock().await.insert(username.clone(), Client { username: username.clone(), sender: tx });

    let mut buffer = [0; 512];
    loop {
        tokio::select! {
            result = stream.read(&mut buffer) => {
                match result {
                    Ok(0) => {
                        println!("Client {} disconnected", username);
                        clients.lock().await.remove(&username);
                        break;
                    }
                    Ok(n) => {
                        let message = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                        println!("Received from {}: {}", username, message);

                        let clients = clients.lock().await;
                        for (_, client) in clients.iter() {
                            if client.username != username {
                                if let Err(e) = client.sender.send(format!("{}: {}", username, message)) {
                                    eprintln!("Failed to send message to {}: {}", client.username, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from stream: {}", e);
                        break;
                    }
                }
            }
            Some(message) = rx.recv() => {
                if let Err(e) = stream.write(message.as_bytes()).await {
                    eprintln!("Failed to write to stream: {}", e);
                    break;
                }
            }
        }
    }
}

async fn run_server() -> io::Result<()> {
    let listener = TcpListener::bind(SERVER).await?;
    println!("Server listening on {}", SERVER);

    let clients = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (mut stream, _) = listener.accept().await?;
        println!("New connection: {}", stream.peer_addr()?);

        let clients = Arc::clone(&clients);
        tokio::spawn(async move {
            let mut buffer = [0; 512];
            if let Ok(n) = stream.read(&mut buffer).await {
                let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                println!("{} connected", username);
                handle_client(stream, clients, username).await;
            }
        });
    }
}

async fn run_client() -> io::Result<()> {
    let retries = 5;
    let delay = 2;

    if let Some(mut stream) = connect_with_retry(ADDR, retries, delay).await {
        println!("Connected to server. Enter your username:");
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        let username = username.trim().to_string();

        if let Err(e) = stream.write(username.as_bytes()).await {
            eprintln!("Failed to send username: {}", e);
            return Ok(());
        }

        println!("Welcome, {}! Type messages to send to other clients.", username);

        let mut buffer = [0; 512];
        loop {
            print!("> ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if let Err(e) = stream.write(input.as_bytes()).await {
                eprintln!("Failed to send data: {}", e);
                break;
            }

            match stream.read(&mut buffer).await {
                Ok(n) => {
                    let response = String::from_utf8_lossy(&buffer[..n]);
                    println!("Response: {}", response);
                }
                Err(e) => {
                    eprintln!("Failed to read from server: {}", e);
                    break;
                }
            }
        }
    } else {
        eprintln!("Failed to connect to server after {} retries", retries);
    }

    Ok(())
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
            if let Err(e) = run_server().await {
                eprintln!("Server error: {}", e);
            }
        }
        "client" => {
            if let Err(e) = run_client().await {
                eprintln!("Client error: {}", e);
            }
        }
        _ => {
            eprintln!("Invalid argument. Use 'server' or 'client'.");
        }
    }
}

async fn connect_with_retry(addr: &str, retries: u32, delay: u64) -> Option<TcpStream> {
    for attempt in 0..retries {
        match TcpStream::connect(addr).await {
            Ok(stream) => {
                println!("Connected to server at {}", addr);
                return Some(stream);
            }
            Err(e) => {
                eprintln!("Attempt {}: Failed to connect to server: {}", attempt + 1, e);
                if attempt < retries - 1 {
                    println!("Retrying in {} seconds...", delay);
                    sleep(Duration::from_secs(delay)).await;
                }
            }
        }
    }
    None
}