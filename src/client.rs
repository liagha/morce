use tokio::time::Duration;
use axo_core::{xeprintln, xprintln, Color};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;
use crate::{Address, Sender, Message};
use crate::errors::Error;
use std::path::Path;
use tokio::fs::File;
use crate::message::{Content, MessageType};

pub struct Client {
    pub username: String,
    pub sender: Sender,
}

impl Client {
    pub async fn run_client(server: Address) -> Result<(), Error> {
        let retries = 5;
        let delay = 5;

        xprintln!("Connecting to server..." => Color::Yellow);

        if let Some(stream) = Self::connect_with_retry(&server, retries, delay).await {
            let username = loop {
                xprintln!("Enter your username:" => Color::BrightBlue);
                let mut username = String::new();
                std::io::stdin().read_line(&mut username).map_err(|e| Error::InputReadFailed(e))?;
                let username = username.trim().to_string();

                if username.len() >= 3 && username.chars().all(|c| c.is_alphanumeric()) {
                    break username;
                } else {
                    xeprintln!("Username must be at least 3 characters long." => Color::Orange);
                }
            };

            xprintln!("Sending username: " => Color::BrightBlue, username => Color::Blue);

            let (mut reader, mut writer) = stream.into_split();

            if let Err(e) = writer.write_all(username.as_bytes()).await {
                xeprintln!("Failed to send username: ", e => Color::Orange);
                return Ok(());
            }

            writer.flush().await.map_err(|e| Error::StreamFlushFailed(e))?;

            xprintln!("Welcome, " => Color::BrightGreen, username => Color::Green, "! Type messages to send to other clients." => Color::BrightGreen);

            let receive_task : tokio::task::JoinHandle<Result<(), Error>> = tokio::spawn(async move {
                let mut buffer = [0; 512];
                loop {
                    match reader.read(&mut buffer).await {
                        Ok(0) => {
                            xprintln!("Server closed connection. Exiting..." => Color::BrightRed);
                            std::process::exit(0);
                        }
                        Ok(n) => {
                            let bytes = &buffer[..n];

                            xprintln!("Size: ", format_size(bytes));

                            match Message::from_bytes(bytes) {
                                Ok(response) => {
                                    match response.content {
                                        Content::Text(text) => {
                                            xprintln!(response.sender, " : ", text);
                                        }
                                        Content::File(file_data) => {
                                            xprintln!(response.sender, " sent a file: ", file_data.name => Color::BrightBlue);

                                            // Save the file to the filesystem with its original name
                                            if let Err(e) = Self::save_file(&file_data.name, &file_data.data).await {
                                                xeprintln!("Failed to save file: ", e => Color::Crimson);
                                            } else {
                                                xprintln!("File saved as: ", file_data.name => Color::BrightGreen);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    break Err(e);
                                }
                            }

                            tokio::io::stdout().flush().await.map_err(|e| Error::StreamFlushFailed(e))?;
                        }
                        Err(e) => {
                            break Err(Error::MessageReceiveFailed(e));
                        }
                    }
                }
            });

            let send_task : tokio::task::JoinHandle<Result<(), Error>> = tokio::spawn(async move {
                loop {
                    tokio::io::stdout().flush().await.map_err(|e| Error::StreamFlushFailed(e))?;
                    let mut input = String::new();

                    std::io::stdin().read_line(&mut input).map_err(|e| Error::InputReadFailed(e))?;

                    let input = input.trim();

                    if input.starts_with("/file ") {
                        let file_path = input.trim_start_matches("/file ");
                        if let Ok(file_data) = Self::read_file(file_path).await {
                            let file_name = Path::new(file_path)
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("unknown_file")
                                .to_string();
                            let message = Message::from_file(file_data, file_name, username.clone(), MessageType::Public);
                            writer.write_all(&message.as_bytes()?).await.map_err(|e| Error::BytesWriteFailed(e))?;
                        } else {
                            xeprintln!("Failed to read file: ", file_path => Color::Orange);
                        }
                    }

                    writer.flush().await.map_err(|e| Error::StreamFlushFailed(e))?;
                }
            });

            tokio::select! {
                result = receive_task => {
                    if let Err(e) = result {
                        xeprintln!("Receive task error: ", e => Color::Crimson);
                        return Err(Error::TaskJoinFailed(e));
                    }
                }
                result = send_task => {
                    if let Err(e) = result {
                        xeprintln!("Send task error: ", e => Color::Crimson);
                        return Err(Error::TaskJoinFailed(e));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn connect_with_retry(server: &Address, retries: u32, delay: u64) -> Option<TcpStream> {
        for attempt in 0..retries {
            xprintln!("Attempt " => Color::BrightYellow, attempt + 1 => Color::BrightYellow,": Connecting to " => Color::Yellow, server => Color::Yellow);

            match TcpStream::connect(server.clone()).await {
                Ok(stream) => {
                    xprintln!("Connected to server successfully " => Color::BrightGreen);
                    return Some(stream);
                }
                Err(e) => {
                    xeprintln!(e => Color::Crimson);

                    if attempt < retries - 1 {
                        xprintln!("Retrying in " => Color::Yellow, delay, " seconds..." => Color::Yellow);
                        sleep(Duration::from_secs(delay)).await;
                    }
                }
            }
        }

        xeprintln!("Failed to connect after " => Color::Crimson, retries, " attempts." => Color::Crimson);
        None
    }

    async fn read_file(file_path: &str) -> Result<Vec<u8>, Error> {
        let path = Path::new(file_path);
        let mut file = File::open(path).await.map_err(|e| Error::InputReadFailed(e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.map_err(|e| Error::InputReadFailed(e))?;
        Ok(buffer)
    }

    async fn save_file(file_name: &str, file_data: &[u8]) -> Result<(), Error> {
        let path = Path::new(file_name);
        let mut file = File::create(path).await.map_err(|e| Error::InputReadFailed(e))?;
        file.write_all(file_data).await.map_err(|e| Error::BytesWriteFailed(e))?;
        Ok(())
    }
}

fn format_size(bytes: &[u8]) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let size = bytes.len() as f64;

    if size < KB {
        format!("{:.0} B", size)
    } else if size < MB {
        format!("{:.2} KB", size / KB)
    } else if size < GB {
        format!("{:.2} MB", size / MB)
    } else if size < TB {
        format!("{:.2} GB", size / GB)
    } else {
        format!("{:.2} TB", size / TB)
    }
}