/// Resolves threaded replies for messages
pub struct ReplyResolver;

impl ReplyResolver {
    /// Create a new reply resolver
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReplyResolver {
    fn default() -> Self {
        Self::new()
    }
}
