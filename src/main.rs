mod server;
mod client;
mod chat;
use tokio::sync::mpsc;
use std::env;
use std::fmt::Formatter;
use axo_core::{xeprintln, xprintln, Color};
use tokio::sync::mpsc::error::SendError;
use crate::client::Client;
use crate::server::Server;

static SERVER: &str = "0.0.0.0:6000";
static ADDR: &str = "192.168.100.195:6000";

use chat::{ChatMessage};

pub enum Error {
    ServerStart(std::io::Error),
    Send(SendError<Message>),
    Read(std::io::Error),
    JoinError(tokio::task::JoinError),
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
            Error::JoinError(err) => {
                write!(f, "Task failed: {}", err)
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum MessageType {
    Private = 0,
    Public = 1
}

pub struct Message {
    sender: String,
    content: String,
    kind: MessageType
}

impl Message {
    pub fn from(msg: &str, from: String, kind: MessageType) -> Self {
        Self {
            sender: from,
            content: msg.to_string(),
            kind,
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        use prost::Message;

        let sender = self.sender.trim().to_string();
        let content = self.content.trim().to_string();

        let chat_message = ChatMessage {
            sender,
            content,
            kind: self.kind as i32,
        };
        let mut buf = Vec::new();
        chat_message.encode(&mut buf).map_err(|_| Error::MessageConversion)?;
        Ok(buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        use prost::Message;

        let chat_message = ChatMessage::decode(bytes).map_err(|_| Error::MessageConversion)?;
        let content = chat_message.content.trim().to_string();

        Ok(Self {
            sender: chat_message.sender,
            content,
            kind: match chat_message.kind {
                0 => MessageType::Private,
                1 => MessageType::Public,
                _ => return Err(Error::MessageConversion),
            },
        })
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