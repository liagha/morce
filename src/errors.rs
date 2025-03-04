use axo_core::Color;
use axo_core::colors::ColoredText;
use crate::{Message};

pub enum Error {
    ServerBindFailed(std::io::Error),
    BytesWriteFailed(std::io::Error),
    MessageSendFailed(tokio::sync::mpsc::error::SendError<Message>),
    MessageReceiveFailed(std::io::Error),
    InputReadFailed(std::io::Error),
    TaskJoinFailed(tokio::task::JoinError),
    FailedToCreateFile(std::io::Error, String),
    StreamFlushFailed(std::io::Error),
    MessageEncodeFailed,
    MessageDecodeFailed,
    ConnectionFailed(u8),
    MessageWriteFailed,
    InvalidUsername,
    HeartBeatTimeOut,
    ClientDisconnected(String, Box<Error>),
    UsernameTaken,
    ServerClosed,
    Disconnected(Box<Error>),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ServerBindFailed(e) => {
                write!(f, "Failed to bind the server to the target address: {}", e)
            }
            Error::MessageSendFailed(e) => {
                write!(f, "Failed to send message: {}", e)
            }
            Error::MessageReceiveFailed(e) => {
                write!(f, "Failed to receive message: {}", e)
            }
            Error::TaskJoinFailed(e) => {
                write!(f, "Task failed to join: {}!", e)
            }
            Error::MessageWriteFailed => {
                write!(f, "Failed to write message to the stream!")
            }
            Error::StreamFlushFailed(e) => {
                write!(f, "Failed to flush the stream: {}!", e)
            }
            Error::MessageEncodeFailed => {
                write!(f, "Failed to encode message to bytes: !")
            }
            Error::MessageDecodeFailed => {
                write!(f, "Failed to decode message from bytes: !")
            }
            Error::InvalidUsername => {
                write!(f, "Invalid username: must be at least 3 alphanumeric characters!")
            }
            Error::ConnectionFailed(retries) => {
                write!(f, "Failed to establish a connection to the server after {} retries!", retries)
            }
            Error::ClientDisconnected(name, e) => {
                write!(f, "{} is disconnected from the server: {}!", name, e)
            }
            Error::InputReadFailed(e) => {
                write!(f, "Failed to read input from io: {}!", e)
            }
            Error::BytesWriteFailed(e) => {
                write!(f, "Failed to write bytes to the stream: {}!", e)
            }
            Error::FailedToCreateFile(e, path) => {
                write!(f, "Failed to create file at {}: {}!", path, e)
            }
            Error::Disconnected(e) => {
                write!(f, "Client is disconnected from the server: {}!", e)
            }
            Error::UsernameTaken => {
                write!(f, "The username is taken!")
            }
            Error::HeartBeatTimeOut => {
                write!(f, "Heartbeat timeout!")
            }
            Error::ServerClosed => {
                write!(f, "Server was closed!")
            }
        }
    }
}

impl core::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = match self {
            Error::ServerBindFailed(e) => {
                format!("Failed to bind the server to the target address: {}", e.to_string().colorize(Color::Crimson))
            }
            Error::MessageSendFailed(e) => {
                format!("Failed to send message: {}", e.to_string().colorize(Color::Crimson))
            }
            Error::MessageReceiveFailed(e) => {
                format!("Failed to receive message: {}", e.to_string().colorize(Color::Crimson))
            }
            Error::TaskJoinFailed(e) => {
                format!("Task failed to join: {}!", e.to_string().colorize(Color::Crimson))
            }
            Error::MessageWriteFailed => {
                "Failed to write message to the stream!".to_string()
            }
            Error::StreamFlushFailed(e) => {
                format!("Failed to flush the stream: {}!", e.to_string().colorize(Color::Crimson))
            }
            Error::MessageEncodeFailed => {
                "Failed to encode message to bytes: !".to_string()
            }
            Error::MessageDecodeFailed => {
                "Failed to decode message from bytes: !".to_string()
            }
            Error::InvalidUsername => {
                "Invalid username: must be at least 3 alphanumeric characters!".to_string()
            }
            Error::ConnectionFailed(retries) => {
                format!("Failed to establish a connection to the server after {} retries!", retries.to_string().colorize(Color::Crimson))
            }
            Error::ClientDisconnected(name, e) => {
                format!("{} is disconnected from the server: {}!", name.to_string().colorize(Color::Crimson), e.to_string().colorize(Color::Crimson))
            }
            Error::InputReadFailed(e) => {
                format!("Failed to read input from io: {}!", e.to_string().colorize(Color::Crimson))
            }
            Error::BytesWriteFailed(e) => {
                format!("Failed to write bytes to the stream: {}!", e.to_string().colorize(Color::Crimson))
            }
            Error::FailedToCreateFile(e, path) => {
                format!("Failed to create file at {}: {}!", path.to_string().colorize(Color::Crimson), e.to_string().colorize(Color::Crimson))
            }
            Error::Disconnected(e) => {
                format!("Client is disconnected from the server: {}!", e.to_string().colorize(Color::Crimson))
            }
            Error::UsernameTaken => {
                "The username is taken!".to_string()
            }
            Error::HeartBeatTimeOut => {
                "Heartbeat timeout!".to_string()
            }
            Error::ServerClosed => {
                "Server was closed!".to_string()
            }
        };

        write!(f, "{} {}", "error: ".colorize(Color::Crimson), error)
    }
}