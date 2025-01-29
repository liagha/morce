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

async fn run_server() -> io::Result<()> {
    let listener = TcpListener::bind(SERVER).await?;
    println!("Server listening on {}", SERVER);

    let clients = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (mut stream, _) = listener.accept().await?;
        println!("New connection: {}", stream.peer_addr()?);

        let mut buffer = [0; 512];
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            println!("Client disconnected during handshake.");
            continue;
        }
        let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
        println!("Received username: {}", username);

        let clients = Arc::clone(&clients);
        tokio::spawn(async move {
            handle_client(stream, clients, username).await;
        });
    }
}

async fn handle_client(stream: TcpStream, clients: Arc<Mutex<HashMap<String, Client>>>, username: String) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    clients.lock().await.insert(username.clone(), Client { username: username.clone(), sender: tx });

    // Split the TCP stream into read and write halves
    let (mut reader, mut writer) = stream.into_split();

    // Spawn a task to handle receiving messages from the client
    let username_clone = username.clone();
    let clients_clone = Arc::clone(&clients);
    let receive_task = tokio::spawn(async move {
        let mut buffer = [0; 512];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    println!("Client {} disconnected", username_clone);
                    clients_clone.lock().await.remove(&username_clone);
                    break;
                }
                Ok(n) => {
                    let message = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    println!("Received from {}: {}", username_clone, message);

                    let clients = clients_clone.lock().await;
                    for (_, client) in clients.iter() {
                        if client.username != username_clone {
                            if let Err(e) = client.sender.send(format!("{}: {}", username_clone, message)) {
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
    });

    // Handle sending messages to the client
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(e) = writer.write_all(message.as_bytes()).await {
                eprintln!("Failed to write to stream: {}", e);
                break;
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = receive_task => {}
        _ = send_task => {}
    }
}

async fn run_client() -> io::Result<()> {
    let retries = 5;
    let delay = 5;

    println!("Connecting to server...");
    if let Some(stream) = connect_with_retry(ADDR, retries, delay).await {
        println!("Connected to server. Enter your username:");
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        let username = username.trim().to_string();
        println!("Sending username: {}", username);

        // Split the TCP stream for separate reading and writing
        let (mut reader, mut writer) = stream.into_split();

        // Send username
        if let Err(e) = writer.write_all(username.as_bytes()).await {
            eprintln!("Failed to send username: {}", e);
            return Ok(());
        }
        writer.flush().await?;

        println!("Welcome, {}! Type messages to send to other clients.", username);

        // Spawn a task to handle receiving messages
        let receive_task = tokio::spawn(async move {
            let mut buffer = [0; 512];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        println!("Server disconnected");
                        break;
                    }
                    Ok(n) => {
                        let response = String::from_utf8_lossy(&buffer[..n]);
                        println!("\rResponse: {}", response);
                        print!("> ");
                        let _ = io::stdout().flush(); // Using let _ to ignore the Result
                    }
                    Err(e) => {
                        eprintln!("Failed to read from server: {}", e);
                        break;
                    }
                }
            }
        });

        // Main loop for sending messages
        let send_task = tokio::spawn(async move {
            loop {
                print!("> ");
                let _ = io::stdout().flush(); // Using let _ to ignore the Result
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    break;
                }

                if let Err(e) = writer.write_all(input.as_bytes()).await {
                    eprintln!("Failed to send data: {}", e);
                    break;
                }
                writer.flush().await?;
            }
            Ok::<_, io::Error>(())
        });

        // Wait for either task to complete
        tokio::select! {
            _ = receive_task => {}
            result = send_task => {
                if let Err(e) = result {
                    eprintln!("Send task error: {:?}", e);
                }
            }
        }
    } else {
        eprintln!("Failed to connect to server after {} retries.", retries);
    }

    Ok(())
}

async fn connect_with_retry(addr: &str, retries: u32, delay: u64) -> Option<TcpStream> {
    for attempt in 0..retries {
        println!("Attempt {}: Connecting to {}...", attempt + 1, addr);
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