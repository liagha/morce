use chrono::{DateTime, Timelike, Utc};
use uuid::Uuid;
use crate::Error;
use crate::time::TimeConversion;

#[derive(Debug, Clone)]
pub struct Message {
    //pub id: Uuid,
    pub sender: String,
    pub content: Content,
    pub kind: MessageType,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChatMessage {
    //#[prost(bytes, tag = "1")]
    //pub id: Vec<u8>,

    #[prost(string, tag = "2")]
    pub sender: String,

    #[prost(oneof = "Content", tags = "3, 4")]
    pub content: Option<Content>,

    #[prost(enumeration = "MessageType", tag = "5")]
    pub kind: i32,

    #[prost(int32, tag = "6")]
    pub timestamp: i32,
}

#[derive(Clone, PartialEq, prost::Oneof)]
pub enum Content {
    #[prost(string, tag = "3")]
    Text(String),

    #[prost(message, tag = "4")]
    File(FileData),
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FileData {
    #[prost(bytes, tag = "1")]
    pub data: Vec<u8>,

    #[prost(string, tag = "2")]
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum MessageType {
    Private = 0,
    Public = 1,
}

impl core::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //let id = self.id;

        match self.content.clone() {
            Content::Text(text) => {
                match self.kind.clone() {
                    MessageType::Private => {
                        let timestamp = self.timestamp.to_local();

                        let time = format!("{}:{}:{}", timestamp.hour(), timestamp.minute(), timestamp.second());
                        write!(f, "[Whisper] {} | {} : {}", time, self.sender, text)
                    }
                    MessageType::Public => {
                        let timestamp = self.timestamp.to_local();

                        let time = format!("{}:{}:{}", timestamp.hour(), timestamp.minute(), timestamp.second());
                        write!(f, "{} | {} : {}", time, self.sender, text)
                    }
                }
            }
            Content::File(file) => {
                match self.kind.clone() {
                    MessageType::Private => {
                        let timestamp = self.timestamp.to_local();

                        let time = format!("{}:{}:{}", timestamp.hour(), timestamp.minute(), timestamp.second());
                        write!(f, "[Whisper] {} | {} : Sent file => {}", time, self.sender, file.name)
                    }
                    MessageType::Public => {
                        let timestamp = self.timestamp.to_local();

                        let time = format!("{}:{}:{}", timestamp.hour(), timestamp.minute(), timestamp.second());
                        write!(f, "{} | {} : Sent file => {}", time, self.sender, file.name)
                    }
                }
            }
        }
    }
}

impl Message {
    pub fn from(msg: &str, from: String, kind: MessageType) -> Self {
        Self {
            //id: Uuid::new_v4(),
            sender: from,
            content: Content::from(msg),
            kind,
            timestamp: Utc::now(),
        }
    }

    pub fn from_file(file_data: Vec<u8>, file_name: String, from: String, kind: MessageType) -> Self {
        Self {
            //id: Uuid::new_v4(),
            sender: from,
            content: Content::File(FileData {
                data: file_data,
                name: file_name,
            }),
            kind,
            timestamp: Utc::now(),
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        use prost::Message;

        let sender = self.sender.trim().to_string();
        println!("┌─ Sender size: {}", format_size(sender.as_bytes().len()));

        match &self.content {
            Content::Text(text) => {
                println!("├─ Content (Text) size: {}", format_size(text.as_bytes().len()));
            }
            Content::File(file_data) => {
                println!("├─ Content (File) size breakdown:");
                println!("├────── File data: {}", format_size(file_data.data.len()));
                println!("├────── File name: {}", format_size(file_data.name.as_bytes().len()));
                println!("├─ Total file content size: {}",
                         format_size(file_data.data.len() + file_data.name.as_bytes().len()));
            }
        }

        println!("├─ MessageType size: {}", format_size(size_of::<i32>()));

        println!("├─ Timestamp size: {}", format_size(2 * size_of::<i64>()));


        let chat_message = ChatMessage {
            //id: self.id.as_bytes().into(),
            sender,
            content: Some(self.content.clone()),
            kind: self.kind as i32,
            timestamp: self.timestamp.to_timestamp(),
        };
        let mut buf = Vec::new();
        chat_message.encode(&mut buf).map_err(|_| Error::MessageConversionFailed)?;

        println!("└─ Total encoded message size: {}", format_size(buf.len()));

        Ok(buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        use prost::Message;

        let chat_message = ChatMessage::decode(bytes).map_err(|_| Error::MessageConversionFailed)?;
        //let id = Uuid::from_bytes(chat_message.id.try_into().unwrap());

        Ok(Self {
            //id,
            sender: chat_message.sender,
            content: chat_message.content.unwrap_or_default(),
            kind: match chat_message.kind {
                0 => MessageType::Private,
                1 => MessageType::Public,
                _ => return Err(Error::MessageConversionFailed),
            },

            timestamp: chat_message.timestamp.to_datetime(),
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

fn format_size(size: usize) -> String {
    let size = size as f64;

    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

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
