use std::collections::HashMap;

use crate::contacts::ContactsIndex;

/// Resolves contact names from handle IDs and identifiers
pub struct ContactResolver {
    contacts_index: ContactsIndex,
    name_cache: HashMap<String, Option<String>>,
    custom_name: Option<String>,
}

impl ContactResolver {
    /// Create a new contact resolver
    pub fn new(contacts_index: ContactsIndex, custom_name: Option<String>) -> Self {
        Self {
            contacts_index,
            name_cache: HashMap::new(),
            custom_name,
        }
    }

    /// Resolve a contact name from an identifier
    ///
    /// Returns the custom name for the database owner if is_from_me is true,
    /// otherwise looks up the contact in the index and caches the result.
    pub fn resolve_name(&mut self, identifier: &str, is_from_me: bool) -> Option<String> {
        // If this is from the database owner, use custom name if provided
        if is_from_me {
            return self.custom_name.clone().or_else(|| Some("Me".to_string()));
        }

        // Check cache first
        if let Some(cached) = self.name_cache.get(identifier) {
            return cached.clone();
        }

        // Look up in contacts index using ContactsIndex.lookup()
        let name = self
            .contacts_index
            .lookup(identifier)
            .map(|n| n.get_display_name().to_string());

        // Cache the result
        self.name_cache.insert(identifier.to_string(), name.clone());

        name
    }
}
