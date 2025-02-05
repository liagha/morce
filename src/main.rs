mod server;
mod client;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use std::env;
use std::fmt::Formatter;
use std::io::Write;
use axo_core::{xeprintln, Color};
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

type Sender = mpsc::UnboundedSender<Message>;
type Receiver = mpsc::UnboundedReceiver<Message>;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        xeprintln!("Usage: ", args[0], " [server|client] [ip (optional)]");
        return;
    }

    let address = if let Some(address) = args.get(3) {
        address
    } else {
        ADDR
    };

    if let Ok(server) = Server::start(address).await {
        match args[1].as_str() {
            "server" => {
                if let Err(err) = server.run().await {
                    xeprintln!("Server error: ", err => Color::Crimson);
                }
            }
            _ => {
                if let Err(err) = Client::run_client(server).await {
                    xeprintln!("Client error: ", err => Color::Crimson);
                }
            }
        }
    }
}