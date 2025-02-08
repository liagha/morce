use std::fmt::Formatter;
use crate::chat::{ChatMessage, MessageContent};
use crate::Error;

#[derive(Clone, PartialEq)]
pub enum Content {
    Text(String),
    File(Vec<u8>),
}

impl Default for Content {
    fn default() -> Self {
        Self::Text("_".to_string())
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        Self::Text("_".to_string())
    }
}

#[derive(Copy, Clone)]
pub enum MessageType {
    Private = 0,
    Public = 1
}

pub struct Message {
    pub sender: String,
    pub content: Content,
    pub kind: MessageType
}

impl From<MessageContent> for Content {
    fn from(msg_content: MessageContent) -> Self {
        match msg_content {
            MessageContent::Text(text) => Content::Text(text),
            MessageContent::File(bytes) => Content::File(bytes),
        }
    }
}

impl From<Content> for MessageContent {
    fn from(content: Content) -> Self {
        match content {
            Content::Text(text) => MessageContent::Text(text),
            Content::File(bytes) => MessageContent::File(bytes),
        }
    }
}

impl std::fmt::Display for Content {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Content::Text(text) => write!(f, "{}", text),
            Content::File(_) => write!(f, "[File content]"),
        }
    }
}

impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Content::Text(text) => write!(f, "Text({})", text),
            Content::File(bytes) => write!(f, "File({} bytes)", bytes.len()),
        }
    }
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl Message {
    pub fn from(msg: &str, from: String, kind: MessageType) -> Self {
        Self {
            sender: from,
            content: Content::from(msg),
            kind,
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        use prost::Message;

        let sender = self.sender.trim().to_string();

        let chat_message = ChatMessage {
            sender,
            content: Some(MessageContent::from(self.content.clone())),
            kind: self.kind as i32,
        };
        let mut buf = Vec::new();
        chat_message.encode(&mut buf).map_err(|_| Error::MessageConversion)?;
        Ok(buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        use prost::Message;

        let chat_message = ChatMessage::decode(bytes).map_err(|_| Error::MessageConversion)?;

        Ok(Self {
            sender: chat_message.sender,
            content: Content::from(chat_message.content.clone().unwrap_or_default()),
            kind: match chat_message.kind {
                0 => MessageType::Private,
                1 => MessageType::Public,
                _ => return Err(Error::MessageConversion),
            },
        })
    }
}
