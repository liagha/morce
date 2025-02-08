mod server;
mod client;
mod chat;
mod message;

use tokio::sync::mpsc;
use std::env;
use std::fmt::Formatter;
use axo_core::{xeprintln, xprintln, Color};
use tokio::sync::mpsc::error::SendError;
use crate::client::Client;
use crate::message::Message;
use crate::server::Server;

static SERVER: &str = "0.0.0.0:6000";
static ADDR: &str = "192.168.100.195:6000";

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