/// Serialization module for converting iMessage types to serializable structs
pub mod attachments;
pub mod chat;
pub mod content;
pub mod message;
pub mod participant;
pub mod relationships;

pub use message::SerializableMessage;
pub use participant::SerializableParticipant;
