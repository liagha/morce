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
    MessageEncodeFailed(prost::EncodeError),
    MessageDecodeFailed(prost::DecodeError),
    ConnectionFailed(u8),
    MessageWriteFailed,
    InvalidUsername,
    ClientDisconnected(String, Box<Error>),
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
            Error::MessageEncodeFailed(e) => {
                write!(f, "Failed to encode message to bytes: {}!", e)
            }
            Error::MessageDecodeFailed(e) => {
                write!(f, "Failed to decode message from bytes: {}!", e)
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
        }
    }
}