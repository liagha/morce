mod server;
mod client;
mod message;
mod errors;
mod time;

use std::hash::{Hash, Hasher};
use tokio::sync::mpsc::error::SendError;
pub use {
    errors::Error,
    client::Client,
    server::Server,
    message::{Message},
    axo_core::{ Color, xprintln, xeprintln },
};
use {
    std::env,
};

static SERVER: &str = "0.0.0.0:6000";
static ADDR: &str = "192.168.100.195:6000";
const BUFFER_SIZE: usize = 8192;

#[derive(Clone)]
pub struct Sender {
    sender: tokio::sync::mpsc::UnboundedSender<Message>,
}

impl Sender {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();

        Self {
            sender: tx
        }
    }
    pub fn from(tx: tokio::sync::mpsc::UnboundedSender<Message>) -> Self {
        Self {
            sender: tx
        }
    }
    pub fn send(&self, message: Message) -> Result<(), SendError<Message>> {
        self.sender.send(message)
    }
}

impl Hash for Sender {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = &self.sender as *const tokio::sync::mpsc::UnboundedSender<Message>;
        ptr.hash(state);
    }
}

pub type Receiver = tokio::sync::mpsc::UnboundedReceiver<Message>;
pub type Address = String;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        xeprintln!("Usage: ", args[0], " [server|client]");
        return Ok(());
    }

    match args[1].as_str() {
        "server" => {
            let address = if let Some(address) = args.get(2) {
                xprintln!("Got ", address, " as server address");
                address
            } else {
                SERVER
            };

            let server = Server::start(address).await?;
            server.run().await?;

            Ok(())
        }
        _ => {
            let address = if let Some(address) = args.get(2) {
                address
            } else {
                ADDR
            };


            match Client::run_client(address.to_string()).await {
                Ok(_) => {
                    Ok(())
                }
                Err(Error::Disconnected(e)) => {
                    xeprintln!(Error::Disconnected(e));
                    std::process::exit(0);
                }
                Err(e) => {
                    xeprintln!("Client error: " => Color::Crimson, e => Color::Red);
                    Err(e)
                }
            }
        }
    }
}