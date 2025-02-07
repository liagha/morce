mod server;
mod client;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use std::env;
use std::fmt::Formatter;
use std::io::Write;
use axo_core::{xeprintln, xprintln, Color};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::error::SendError;
use crate::client::Client;
use crate::server::Server;

static SERVER: &str = "0.0.0.0:6000";
static ADDR: &str = "192.168.100.195:6000";

pub enum Error {
    ServerStart(std::io::Error),
    Send(SendError<Message>),
    Read(std::io::Error),
    Write,
    Flush,
    MessageConversion,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ServerStart(e) => {
                write!(f, "Server start error: {}", e)
            }
            Error::Send(e) => {
                write!(f, "Message sending error: {}", e)
            }
            Error::Read(e) => {
                write!(f, "Message receiving error: {}", e)
            }
            Error::Write => {
                write!(f, "Message writing error")
            }
            Error::MessageConversion => {
                write!(f, "Message conversion error")
            }
            Error::Flush => {
                write!(f, "Writer flush error")
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum MessageType {
    Private = 0,
    Public = 1
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    content: String,
    kind: MessageType
}

impl Message {
    pub fn from(msg: &str, kind: MessageType) -> Self {
        Self {
            content: msg.to_string(),
            kind,
        }
    }
    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        match serde_json::to_string(&self) {
            Ok(json) => Ok(json.into_bytes()),
            Err(_) => Err(Error::MessageConversion)
        }
    }
}

pub type Sender = mpsc::UnboundedSender<Message>;
pub type Receiver = mpsc::UnboundedReceiver<Message>;
pub type Address = String;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        xeprintln!("Usage: ", args[0], " [server|client]");
        return;
    }



    match args[1].as_str() {
        "server" => {
            let address = if let Some(address) = args.get(2) {
                xprintln!("Got ", address, " as server address");
                address
            } else {
                SERVER
            };

            if let Ok(server) = Server::start(address).await {
                if let Err(err) = server.run().await {
                    xeprintln!("Server error: ", err => Color::Crimson);
                }
            } else {
                println!("shit");
            }
        }
        _ => {
            xprintln!("Start");
            let address = if let Some(address) = args.get(2) {
                address
            } else {
                ADDR
            };

            if let Err(e) = Client::run_client(address.to_string()).await {
                xeprintln!("Client error: ", e => Color::Crimson);
            }
        }
    }
}