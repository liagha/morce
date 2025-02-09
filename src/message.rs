// message.rs
use crate::Error;

pub struct Message {
    pub sender: String,
    pub content: Content,
    pub kind: MessageType,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChatMessage {
    #[prost(string, tag = "1")]
    pub sender: String,

    #[prost(oneof = "Content", tags = "2, 3")]
    pub content: Option<Content>,

    #[prost(enumeration = "MessageType", tag = "4")]
    pub kind: i32,
}

#[derive(Clone, PartialEq, prost::Oneof)]
pub enum Content {
    #[prost(string, tag = "2")]
    Text(String),

    #[prost(message, tag = "3")]
    File(FileData),
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FileData {
    #[prost(string, tag = "1")]
    pub name: String,

    #[prost(bytes, tag = "2")]
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum MessageType {
    Private = 0,
    Public = 1,
}

impl Message {
    pub fn from(msg: &str, from: String, kind: MessageType) -> Self {
        Self {
            sender: from,
            content: Content::from(msg),
            kind,
        }
    }

    pub fn from_file(file_data: Vec<u8>, file_name: String, from: String, kind: MessageType) -> Self {
        Self {
            sender: from,
            content: Content::File(FileData {
                data: file_data,
                name: file_name,
            }),
            kind,
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        use prost::Message;

        let sender = self.sender.trim().to_string();

        let chat_message = ChatMessage {
            sender,
            content: Some(self.content.clone()),
            kind: self.kind as i32,
        };
        let mut buf = Vec::new();
        chat_message.encode(&mut buf).map_err(|_| Error::MessageConversionFailed)?;
        Ok(buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        use prost::Message;

        let chat_message = ChatMessage::decode(bytes).map_err(|_| Error::MessageConversionFailed)?;

        Ok(Self {
            sender: chat_message.sender,
            content: chat_message.content.unwrap_or_default(),
            kind: match chat_message.kind {
                0 => MessageType::Private,
                1 => MessageType::Public,
                _ => return Err(Error::MessageConversionFailed),
            },
        })
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::Text("".to_string())
    }
}

impl std::fmt::Display for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Content::Text(text) => write!(f, "{}", text),
            Content::File(_) => write!(f, "[File content]"),
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

impl MessageType {
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Private => "PRIVATE",
            Self::Public => "PUBLIC",
        }
    }
    pub fn from_str_name(value: &str) -> Option<Self> {
        match value {
            "PRIVATE" => Some(Self::Private),
            "PUBLIC" => Some(Self::Public),
            _ => None,
        }
    }
}