// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChatMessage {
    #[prost(string, tag = "1")]
    pub content: ::prost::alloc::string::String,
    #[prost(enumeration = "MessageType", tag = "2")]
    pub kind: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MessageType {
    Private = 0,
    Public = 1,
}
impl MessageType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Private => "PRIVATE",
            Self::Public => "PUBLIC",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "PRIVATE" => Some(Self::Private),
            "PUBLIC" => Some(Self::Public),
            _ => None,
        }
    }
}
