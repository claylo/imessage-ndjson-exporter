use std::collections::HashMap;
use imessage_database::tables::messages::Message;

/// Resolves tapbacks (reactions) for messages
pub struct TapbackResolver {
    /// Maps message GUID -> list of tapback messages
    tapback_map: HashMap<String, Vec<Message>>,
}

impl TapbackResolver {
    /// Create a new empty tapback resolver
    pub fn new() -> Self {
        Self {
            tapback_map: HashMap::new(),
        }
    }

    /// Add a tapback message to the resolver
    ///
    /// This should be called for all tapback messages during the initial pass
    pub fn add_tapback(&mut self, associated_guid: String, tapback_message: Message) {
        self.tapback_map
            .entry(associated_guid)
            .or_default()
            .push(tapback_message);
    }

    /// Get all tapbacks for a given message GUID
    pub fn get_tapbacks(&self, message_guid: &str) -> Option<&Vec<Message>> {
        self.tapback_map.get(message_guid)
    }
}

impl Default for TapbackResolver {
    fn default() -> Self {
        Self::new()
    }
}
