use std::collections::HashMap;

/// Counts threaded replies per message GUID.
///
/// Built during the cache phase by scanning all messages for `thread_originator_guid`.
#[derive(Default)]
pub struct ReplyResolver {
    /// Maps message GUID -> number of replies
    reply_counts: HashMap<String, i64>,
}

impl ReplyResolver {
    /// Create a new reply resolver from a pre-built counts map.
    pub fn new(reply_counts: HashMap<String, i64>) -> Self {
        Self { reply_counts }
    }

    /// Get the number of replies for a given message GUID.
    pub fn get_reply_count(&self, message_guid: &str) -> i64 {
        self.reply_counts.get(message_guid).copied().unwrap_or(0)
    }
}
