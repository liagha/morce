use std::path::Path;
use tokio::{
    time::{sleep, Duration},
    net::TcpStream,
    io::{AsyncReadExt, AsyncWriteExt},
    fs::File,
};
use crate::{message::{Content, MessageType}, {Address, Sender, Message}, errors::Error, BUFFER_SIZE};
use axo_core::{xeprintln, xprintln, Color};

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
                let mut length_buffer = [0; 8]; // Buffer for the 8-byte message length

                loop {
                    // Read the message length first (8-byte u64)
                    match reader.read_exact(&mut length_buffer).await {
                        Ok(0) => {
                            xprintln!("Server closed connection. Exiting..." => Color::BrightRed);
                            std::process::exit(0);
                        }
                        Ok(_n) => {
                            // Convert 8 bytes to u64 (message length)
                            let message_length = u64::from_be_bytes(length_buffer);

                            // Create a buffer to hold the entire message
                            let mut message_buffer = vec![0; message_length as usize];
                            let mut bytes_read = 0;

                            // Read message in chunks until we've read the full message
                            while bytes_read < message_length as usize {
                                let remaining = message_length as usize - bytes_read;
                                let chunk_size = std::cmp::min(remaining, BUFFER_SIZE);

                                match reader.read(&mut message_buffer[bytes_read..(bytes_read + chunk_size)]).await {
                                    Ok(n) if n > 0 => {
                                        bytes_read += n;
                                    }
                                    Ok(0) => {
                                        xprintln!("Server closed connection. Exiting..." => Color::BrightRed);
                                        std::process::exit(0);
                                    }
                                    Ok(_) => {
                                        continue;
                                    }
                                    Err(e) => {
                                        return Err(Error::MessageReceiveFailed(e));
                                    }
                                }
                            }

                            // Process the complete message
                            match Message::from_bytes(&message_buffer) {
                                Ok(response) => {
                                    xprintln!(response);

                                    match response.content {
                                        Content::Text(_text) => {
                                            // Handle text message
                                        }
                                        Content::File(file_data) => {
                                            if let Err(e) = Self::save_file(&file_data.name, &file_data.data).await {
                                                return Err(e);
                                            } else {
                                                xprintln!("File saved as: ", file_data.name => Color::BrightGreen);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }

                            tokio::io::stdout().flush().await.map_err(|e| Error::StreamFlushFailed(e))?;
                        }
                        Err(e) => {
                            return Err(Error::MessageReceiveFailed(e));
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
                        let file_name = Path::new(file_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("unknown_file")
                            .to_string();

                        let mut file = File::open(file_path).await.map_err(|e| Error::InputReadFailed(e))?;
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer).await.map_err(|e| Error::InputReadFailed(e))?;

                        let message = Message::from_file(buffer, file_name.clone(), username.clone(), MessageType::Public);
                        let message_bytes = message.as_bytes()?;
                        let message_len = message_bytes.len() as u64;

                        writer.write_all(&message_len.to_be_bytes()).await
                            .map_err(|e| Error::BytesWriteFailed(e))?;

                        xprintln!("Sending chunks for file {", file_name, "} : ", message_len);

                        for chunk in message_bytes.chunks(BUFFER_SIZE) {
                            writer.write_all(chunk).await
                                .map_err(|e| Error::BytesWriteFailed(e))?;
                        }
                    }
                    else {
                        let message = Message::from(input, username.clone(), MessageType::Public);
                        let message_bytes = message.as_bytes()?;
                        let message_len = message_bytes.len() as u64;

                        // Send message length as 8-byte u64
                        writer.write_all(&message_len.to_be_bytes()).await
                            .map_err(|e| Error::BytesWriteFailed(e))?;

                        // Send message in chunks
                        for chunk in message_bytes.chunks(BUFFER_SIZE) {
                            writer.write_all(chunk).await
                                .map_err(|e| Error::BytesWriteFailed(e))?;
                        }
                    }

                    writer.flush().await.map_err(|e| Error::StreamFlushFailed(e))?;
                }
            });

            tokio::select! {
                result = receive_task => {
                    if let Err(e) = result {
                        return Err(Error::TaskJoinFailed(e));
                    }
                }
                result = send_task => {
                    if let Err(e) = result {
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

    async fn save_file(file_name: &str, file_data: &[u8]) -> Result<(), Error> {
        let path = Path::new(file_name);
        let mut file = File::create(path).await.map_err(|e| Error::FailedToCreateFile(e, path.as_os_str().to_string_lossy().to_string()))?;
        file.write_all(file_data).await.map_err(|e| Error::BytesWriteFailed(e))?;
        Ok(())
    }
}
