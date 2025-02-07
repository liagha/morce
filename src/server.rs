use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use axo_core::{xeprintln, xprintln, Color};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::sleep;
use crate::{Error, Message, MessageType};
use crate::client::{handle_client, Client};

pub struct Server {
    pub address: String,
    pub listener: TcpListener,
    pub clients: Arc<Mutex<HashMap<String, Client>>>,
}

impl Server {
    pub async fn start(address: &str) -> Result<Self, Error> {
        xprintln!("Attempting to bind to address: " => Color::Yellow, address => Color::Yellow);

        let listener = match TcpListener::bind(address).await {
            Ok(listener) => listener,
            Err(err) => {
                xeprintln!("Failed to bind to address: ", err => Color::Crimson);
                return Err(Error::ServerStart(err));
            }
        };

        xprintln!("Server listening on " => Color::BrightGreen, address => Color::Green);

        Ok(Self {
            address: address.to_string(),
            listener,
            clients: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    pub async fn run(&self) -> Result<(), Error> {
        loop {
            tokio::select! {
                Ok((mut stream, addr)) = self.listener.accept() => {
                    xprintln!("New connection from " => Color::BrightBlue, addr => Color::Blue ; Debug);

                    let mut buffer = [0; 512];
                    let n = stream.read(&mut buffer).await.map_err(|err| Error::Read(err))?;

                    //xprintln!("Read " => Color::Cyan, n, " bytes from stream" => Color::Cyan);

                    if n == 0 {
                        continue;
                    }

                    let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
                    let clients = Arc::clone(&self.clients);

                    tokio::spawn(async move {
                        return handle_client(stream, clients, username).await;
                    });
                }
                _ = tokio::signal::ctrl_c() => {
                    xprintln!("Shutting down server..." => Color::BrightRed);

                    let mut clients = self.clients.lock().await;

                    for (_, client) in clients.iter() {
                        let message = Message::from("Server is shutting down...", MessageType::Public);

                        client.sender.send(message).map_err(|err| Error::Send(err))?;
                    }

                    sleep(Duration::from_secs(1)).await;

                    clients.clear();

                    break;
                }
            }
        }

        Ok(())
    }
}
