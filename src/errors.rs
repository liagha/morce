use crate::Message;

pub enum Error {
    ServerBindFailed(std::io::Error),
    BytesWriteFailed(std::io::Error),
    MessageSendFailed(tokio::sync::mpsc::error::SendError<Message>),
    MessageReceiveFailed(std::io::Error),
    InputReadFailed(std::io::Error),
    TaskJoinFailed(tokio::task::JoinError),
    FailedToCreateFile(std::io::Error, String),
    MessageIDConversionFailed,
    MessageWriteFailed,
    StreamFlushFailed(std::io::Error),
    MessageConversionFailed,
    InvalidUsername,
    ConnectionFailed,
    ClientDisconnected,
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
                write!(f, "Task failed to join: {}", e)
            }
            Error::MessageWriteFailed => {
                write!(f, "Failed to write message to the stream")
            }
            Error::StreamFlushFailed(e) => {
                write!(f, "Failed to flush the stream: {}", e)
            }
            Error::MessageConversionFailed => {
                write!(f, "Failed to convert message to/from bytes")
            }
            Error::InvalidUsername => {
                write!(f, "Invalid username: must be at least 3 alphanumeric characters")
            }
            Error::ConnectionFailed => {
                write!(f, "Failed to establish a connection to the server")
            }
            Error::ClientDisconnected => {
                write!(f, "Client disconnected from the server")
            }
            Error::InputReadFailed(e) => {
                write!(f, "Failed to read input from io: {}", e)
            }
            Error::BytesWriteFailed(e) => {
                write!(f, "Failed to write bytes to the stream: {}", e)
            }
            Error::FailedToCreateFile(e, path) => {
                write!(f, "Failed to create file at {}: {}", path, e)
            }
            Error::MessageIDConversionFailed => {
                write!(f, "Failed to convert the message id")
            }
        }
    }
}