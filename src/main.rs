mod server;
mod client;
mod message;
mod errors;
mod time;

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

pub type Sender = tokio::sync::mpsc::UnboundedSender<Message>;
pub type Receiver = tokio::sync::mpsc::UnboundedReceiver<Message>;
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

            match Server::start(address).await {
                Ok(server) => {
                    if let Err(e) = server.run().await {
                        xeprintln!("Server error: ", e => Color::Crimson);
                    }
                }
                Err(e) => {
                    xeprintln!("Server start error: ", e => Color::Crimson);
                }
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
