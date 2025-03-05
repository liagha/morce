use chrono::{DateTime, Utc};
use orbyte::Serialize;
use orbyte::Deserialize;
use crate::Error;
use crate::time::TimeConversion;
use broccli::{ xprintln };

#[derive(Debug, Clone)]
pub struct Message {
    pub sender: String,
    pub content: Content,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, PartialEq, orbyte::Orbyte)]
pub struct ChatMessage {
    pub sender: String,

    pub content: Option<Content>,

    pub timestamp: Option<i32>,
}

#[derive(Clone, PartialEq, orbyte::Orbyte, Debug)]
pub enum Content {
    Text(String),

    File(FileData),

    Signal(u8)
}

#[derive(Clone, PartialEq, orbyte::Orbyte, Debug)]
pub struct FileData {
    pub data: Vec<u8>,

    pub name: String,
}

impl core::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let timestamp = self.timestamp.to_local();

        let time = timestamp.to_str();

        match self.content.clone() {
            Content::Text(text) => {
                write!(f, "{} | {} : {}", time, self.sender, text)
            }
            Content::File(file) => {
                write!(f, "{} | {} : Sent file => {}", time, self.sender, file.name)
            }
            Content::Signal(code) => {
                match code {
                    0 => {
                        write!(f, "{} | {} is still alive!", time, self.sender)
                    }
                    1 => {
                        write!(f, "{} | {} is still alive!", time, self.sender)
                    }
                    _ => {
                        write!(f, "{} | {} sent a signal which is still not implemented!", time, self.sender)
                    }
                }
            }
        }
    }
}

impl Message {
    pub fn from(msg: &str, from: &String) -> Self {
        Self {
            sender: from.to_string(),
            content: Content::from(msg),
            timestamp: Utc::now(),
        }
    }

    pub fn from_file(file_data: Vec<u8>, file_name: String, from: &String) -> Self {
        Self {
            sender: from.to_string(),
            content: Content::File(FileData {
                data: file_data,
                name: file_name,
            }),
            timestamp: Utc::now(),
        }
    }

    pub fn from_code(code: u8, from: &String) -> Self {
        Self {
            sender: from.to_string(),
            content: Content::Signal(code),
            timestamp: Utc::now(),
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        let sender = self.sender.trim().to_string();

        xprintln!("┌─ Sender size: ", format_size(sender.as_bytes().len()));

        let chat_message =
            ChatMessage {
                sender,
                content: Some(self.content.clone()),
                timestamp: Some(self.timestamp.to_timestamp()),
            };

        let buf = chat_message.serialize();

        match &self.content {
            Content::Text(text) => {
                xprintln!("├─ Content (Text) size: ", format_size(text.as_bytes().len()));
            }
            Content::Signal(_code) => {
                xprintln!("├─ Content (Signal) size: 1");
            }
            Content::File(file_data) => {
                xprintln!("├─ Content (File) size breakdown:");
                xprintln!("├────── File data: ", format_size(file_data.data.len()));
                xprintln!("├────── File name: ", format_size(file_data.name.as_bytes().len()));
                xprintln!("├─ Total file content size: ",
                         format_size(file_data.data.len() + file_data.name.as_bytes().len()));
            }
        }

        xprintln!("├─ MessageType size: ", format_size(size_of::<i32>()));

        xprintln!("├─ Timestamp size: ", format_size(2 * size_of::<i64>()));
        xprintln!("└─ Total encoded message size: ", format_size(buf.len()));

        Ok(buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let chat_message = ChatMessage::deserialize(bytes).ok_or(Error::MessageDecodeFailed)?;

        Ok(Self {
            sender: chat_message.sender,
            content: chat_message.content.unwrap_or_default(),
            timestamp: {
                match chat_message.timestamp {
                    Some(timestamp) => timestamp.to_datetime(),
                    None => DateTime::default(),
                }
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
            Content::Signal(code) => write!(f, "Signal: code {}", code),
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

#[allow(dead_code)]
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