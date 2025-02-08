#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChatMessage {
    #[prost(string, tag = "1")]
    pub sender: String,
    #[prost(string, tag = "2")]
    pub content: String,
    #[prost(enumeration = "MessageType", tag = "3")]
    pub kind: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum MessageType {
    Private = 0,
    Public = 1,
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
