# MIT Migration + Workspace Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `imessage-database` (GPL-3.0) with `imessage-db` + `imessage-core` (MIT), split into a workspace with a reusable library crate and a thin CLI binary using librebar.

**Architecture:** The project becomes a Cargo workspace with two crates: `crates/imessage-ndjson-core` (MIT library — all export logic, serialization, converters, resolvers) and the root binary (thin CLI using librebar for app bootstrap + clap for args, depends on core). The library crate replaces all `imessage-database` usage with `imessage-db` for DB access and `imessage-core` for typedstream decoding and date conversion.

**Tech Stack:** Rust 2021, `imessage-db` 0.1.0, `imessage-core` 0.1.0, `librebar` 0.1.0 (crates.io), `rusqlite` 0.32 (bundled), `clap` 4.x, `serde`/`serde_json`

---

## File Structure

### New workspace layout

```
Cargo.toml                          (workspace root — defines members)
rust-toolchain.toml                 (unchanged)
Justfile                            (unchanged, commands work from root)
src/
  main.rs                           (REWRITE — thin CLI using librebar + core)
  cli.rs                            (MODIFY — add CommonArgs flatten, keep domain args)
crates/
  imessage-ndjson-core/
    Cargo.toml                      (CREATE — MIT library, deps on imessage-db, imessage-core)
    LICENSE-MIT                     (CREATE)
    src/
      lib.rs                        (CREATE — module declarations, re-exports)
      exporter.rs                   (MOVE+REWRITE — use MessageRepository instead of streaming)
      db.rs                         (CREATE — thin wrapper around MessageRepository)
      attachment_manager.rs         (MOVE+MODIFY — remove imessage-database Attachment refs)
      avatar_manager.rs             (MOVE — no changes needed)
      contacts.rs                   (MOVE+MODIFY — use imessage-core phone normalization)
      resolvers/
        mod.rs                      (MOVE — unchanged)
        contacts.rs                 (MOVE — unchanged)
        tapbacks.rs                 (MOVE+REWRITE — use local Message type, not imessage-database's)
        replies.rs                  (MOVE — unchanged)
      serialization/
        mod.rs                      (MOVE — unchanged)
        message.rs                  (MOVE+MODIFY — dates are already Unix ms)
        content.rs                  (MOVE+MODIFY — build from typedstream, not BubbleComponent)
        chat.rs                     (MOVE — unchanged)
        relationships.rs            (MOVE — unchanged)
        attachments.rs              (MOVE+MODIFY — use local Attachment wrapper)
        participant.rs              (MOVE — unchanged)
      converters/
        mod.rs                      (MOVE — unchanged)
        models.rs                   (MOVE — unchanged)
        image.rs                    (MOVE — unchanged)
        video.rs                    (MOVE — unchanged)
        audio.rs                    (MOVE — unchanged)
        sticker.rs                  (MOVE — unchanged)
        common.rs                   (MOVE — unchanged)
      test_utils/
        mod.rs                      (MOVE — unchanged)
        database.rs                 (MOVE+MODIFY — rusqlite 0.32)
        assertions.rs               (MOVE — unchanged)
        fixtures.rs                 (MOVE — unchanged)
```

### Files deleted from root `src/`

All files except `main.rs` and `cli.rs` move into `crates/imessage-ndjson-core/src/`. The root `src/lib.rs` is deleted — the root crate is binary-only.

---

## Task 1: Create workspace structure and move files

**Files:**
- Create: `Cargo.toml` (workspace root — replaces current)
- Create: `crates/imessage-ndjson-core/Cargo.toml`
- Create: `crates/imessage-ndjson-core/LICENSE-MIT`
- Create: `crates/imessage-ndjson-core/src/lib.rs`
- Move: all modules from `src/` to `crates/imessage-ndjson-core/src/`
- Modify: `src/main.rs` (temporary — import from new crate)
- Delete: `src/lib.rs`

This task creates the workspace skeleton and moves files. The code still uses `imessage-database` at this point — we just want `cargo check` to pass with the new layout.

- [ ] **Step 1: Create workspace root Cargo.toml**

Back up the current Cargo.toml, then replace it with the workspace definition. The root crate becomes the binary; the library lives in `crates/`.

```toml
[workspace]
members = [".", "crates/imessage-ndjson-core"]
resolver = "2"

[package]
name = "imessage-ndjson-exporter"
version = "0.1.0"
edition = "2021"
authors = ["Clay"]
description = "Export iMessage data to NDJSON format"
license = "MIT"

[dependencies]
# The core library
imessage-ndjson-core = { path = "crates/imessage-ndjson-core" }

# CLI framework
librebar = { version = "0.1", features = ["cli", "logging", "crash"] }

# CLI (still needed for domain-specific args)
clap = { version = "4.4", features = ["derive", "cargo"] }

# Error handling
anyhow = "1.0"

[[bin]]
name = "imessage-ndjson-exporter"
path = "src/main.rs"

[profile.release]
lto = true
codegen-units = 1
strip = true
```

- [ ] **Step 2: Create library crate Cargo.toml**

```bash
mkdir -p crates/imessage-ndjson-core/src
```

Create `crates/imessage-ndjson-core/Cargo.toml`:

```toml
[package]
name = "imessage-ndjson-core"
version = "0.1.0"
edition = "2021"
authors = ["Clay"]
description = "Core library for iMessage NDJSON export"
license = "MIT"

[dependencies]
# iMessage database access (MIT)
imessage-db = "0.1"
imessage-core = "0.1"

# Database (match imessage-db's version)
rusqlite = { version = "0.32", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Date/time
chrono = "0.4"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Progress indicators
indicatif = "0.17"

# Crypto/encoding for attachments
sha2 = "0.10"
hex = "0.4"
base64 = "0.22"
flate2 = "1.0"
zstd = "0.13"

# Plist parsing
plist = "^1.8"

[features]
test-utils = []

[dev-dependencies]
tempfile = "3.8"
assert-json-diff = "2.0"
predicates = "3.0"
assert_fs = "1.1"
assert_cmd = "2.0"
plist = "^1.8"
```

- [ ] **Step 3: Create MIT license file**

Create `crates/imessage-ndjson-core/LICENSE-MIT` with standard MIT license text, copyright Clay.

- [ ] **Step 4: Move source files into library crate**

```bash
# Move everything except main.rs and cli.rs
cp src/exporter.rs crates/imessage-ndjson-core/src/
cp src/attachment_manager.rs crates/imessage-ndjson-core/src/
cp src/avatar_manager.rs crates/imessage-ndjson-core/src/
cp src/contacts.rs crates/imessage-ndjson-core/src/

cp -r src/resolvers crates/imessage-ndjson-core/src/
cp -r src/serialization crates/imessage-ndjson-core/src/
cp -r src/converters crates/imessage-ndjson-core/src/
cp -r src/test_utils crates/imessage-ndjson-core/src/

# Remove moved files from root src/
rm src/exporter.rs src/attachment_manager.rs src/avatar_manager.rs src/contacts.rs
rm -rf src/resolvers src/serialization src/converters src/test_utils
rm src/lib.rs
```

- [ ] **Step 5: Create library crate lib.rs**

Create `crates/imessage-ndjson-core/src/lib.rs`:

```rust
pub mod attachment_manager;
pub mod avatar_manager;
pub mod contacts;
pub mod converters;
pub mod exporter;
pub mod resolvers;
pub mod serialization;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use exporter::NdjsonExporter;
```

- [ ] **Step 6: Update src/main.rs to use the library crate**

Temporary bridge — change `use imessage_ndjson_exporter::` to `use imessage_ndjson_core::`:

```rust
use anyhow::{Context, Result};
use clap::Parser;
use imessage_ndjson_core::{NdjsonExporter, attachment_manager::CompressionMode};

mod cli;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // ... rest stays the same for now
}
```

- [ ] **Step 7: Update internal `use crate::` paths in moved files**

All `use crate::` references in the moved files should still work since they're now inside the library crate. Verify no breakage.

- [ ] **Step 8: Run `cargo check` to verify workspace compiles**

Run: `cargo check --workspace 2>&1`

Expected: Compiles with the existing `imessage-database` dep still in the library crate's Cargo.toml (add it temporarily if needed). This step validates the workspace structure before we start swapping dependencies.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: split into workspace with imessage-ndjson-core library crate"
```

---

## Task 2: Replace database access layer

**Files:**
- Create: `crates/imessage-ndjson-core/src/db.rs`
- Modify: `crates/imessage-ndjson-core/src/exporter.rs`
- Modify: `crates/imessage-ndjson-core/Cargo.toml`

Replace `imessage-database`'s `get_connection()`, `Table::stream()`, `Cacheable::cache()`, and `QueryContext` with `imessage-db`'s `MessageRepository` and paginated queries.

- [ ] **Step 1: Create db.rs wrapper module**

This module wraps `imessage_db::imessage::repository::MessageRepository` and provides the query methods the exporter needs. It translates between `imessage-db` entity types and our internal types.

Create `crates/imessage-ndjson-core/src/db.rs`:

```rust
use anyhow::Result;
use imessage_db::imessage::entities::{
    Attachment, Chat, Handle, Message,
};
use imessage_db::imessage::repository::MessageRepository;
use imessage_db::imessage::types::{ChatQueryParams, HandleQueryParams, MessageQueryParams, SortOrder};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

/// Thin wrapper around MessageRepository providing the query patterns
/// needed by the exporter.
pub struct Database {
    repo: MessageRepository,
}

impl Database {
    /// Open a read-only connection to the iMessage database.
    pub fn open(path: &Path) -> Result<Self> {
        let repo = MessageRepository::open(path.to_path_buf())
            .map_err(|e| anyhow::anyhow!("Failed to open iMessage database: {}", e))?;
        Ok(Self { repo })
    }

    /// Load all chats into a HashMap keyed by rowid.
    pub fn load_chats(&self) -> Result<HashMap<i64, Chat>> {
        let mut all_chats = HashMap::new();
        let mut offset = 0i64;
        let limit = 500i64;

        loop {
            let params = ChatQueryParams {
                with_participants: false,
                with_last_message: false,
                with_archived: true,
                offset,
                limit: Some(limit),
                ..Default::default()
            };

            let (chats, _total) = self.repo.get_chats(params)
                .map_err(|e| anyhow::anyhow!("Failed to load chats: {}", e))?;

            if chats.is_empty() {
                break;
            }

            for chat in chats {
                all_chats.insert(chat.rowid, chat);
            }

            offset += limit;
        }

        Ok(all_chats)
    }

    /// Load all handles into a HashMap keyed by rowid.
    pub fn load_handles(&self) -> Result<HashMap<i64, Handle>> {
        let mut all_handles = HashMap::new();
        let mut offset = 0i64;
        let limit = 1000i64;

        loop {
            let params = HandleQueryParams {
                address: None,
                offset,
                limit,
            };

            let (handles, _total) = self.repo.get_handles(params)
                .map_err(|e| anyhow::anyhow!("Failed to load handles: {}", e))?;

            if handles.is_empty() {
                break;
            }

            for handle in handles {
                all_handles.insert(handle.rowid, handle);
            }

            offset += limit;
        }

        Ok(all_handles)
    }

    /// Load messages for a specific chat, sorted by date ascending.
    /// Returns messages in pages to avoid loading everything into memory.
    pub fn messages_for_chat(
        &self,
        chat_guid: &str,
        after: Option<i64>,
        before: Option<i64>,
    ) -> Result<Vec<Message>> {
        let mut all_messages = Vec::new();
        let mut offset = 0i64;
        let limit = 500i64;

        loop {
            let params = MessageQueryParams {
                chat_guid: Some(chat_guid.to_string()),
                offset,
                limit,
                after,
                before,
                with_chats: false,
                with_chat_participants: false,
                with_attachments: true,
                sort: SortOrder::Asc,
                ..Default::default()
            };

            let (messages, _total) = self.repo.get_messages(params)
                .map_err(|e| anyhow::anyhow!("Failed to load messages: {}", e))?;

            if messages.is_empty() {
                break;
            }

            let count = messages.len();
            all_messages.extend(messages);

            if (count as i64) < limit {
                break;
            }

            offset += limit;
        }

        Ok(all_messages)
    }

    /// Load all tapback messages (reactions) keyed by associated_message_guid.
    pub fn load_tapbacks(&self) -> Result<HashMap<String, Vec<Message>>> {
        let mut tapbacks: HashMap<String, Vec<Message>> = HashMap::new();
        let mut offset = 0i64;
        let limit = 1000i64;

        loop {
            let params = MessageQueryParams {
                offset,
                limit,
                with_attachments: false,
                with_chats: false,
                with_chat_participants: false,
                sort: SortOrder::Asc,
                ..Default::default()
            };

            let (messages, _total) = self.repo.get_messages(params)
                .map_err(|e| anyhow::anyhow!("Failed to load messages for tapbacks: {}", e))?;

            if messages.is_empty() {
                break;
            }

            let count = messages.len();
            for msg in messages {
                if msg.associated_message_type.is_some() {
                    if let Some(ref guid) = msg.associated_message_guid {
                        tapbacks.entry(guid.clone()).or_default().push(msg);
                    }
                }
            }

            if (count as i64) < limit {
                break;
            }

            offset += limit;
        }

        Ok(tapbacks)
    }

    /// Get participants for a specific chat.
    pub fn chat_participants(&self, chat_rowid: i64) -> Result<Vec<Handle>> {
        self.repo
            .get_chat_participants(chat_rowid)
            .map_err(|e| anyhow::anyhow!("Failed to get chat participants: {}", e))
    }

    /// Access the underlying repository for direct queries.
    pub fn repo(&self) -> &MessageRepository {
        &self.repo
    }
}
```

- [ ] **Step 2: Add db module to lib.rs**

Add `pub mod db;` to `crates/imessage-ndjson-core/src/lib.rs`.

- [ ] **Step 3: Rewrite exporter.rs imports and connection setup**

Replace the top imports in `exporter.rs`:

Old:
```rust
use imessage_database::{
    message_types::variants::Announcement,
    tables::{
        attachment::Attachment,
        chat::Chat,
        chat_handle::ChatToHandle,
        handle::Handle,
        messages::{Message, models::GroupAction},
        table::{Cacheable, Table, get_connection},
    },
    util::{platform::Platform, query_context::QueryContext},
};
use rusqlite::Connection;
```

New:
```rust
use imessage_db::imessage::entities::{Attachment, Chat, Handle, Message};
use crate::db::Database;
```

- [ ] **Step 4: Rewrite NdjsonExporter to use Database**

Replace the `database_path` field usage. The `export()` method changes from:
```rust
let db = get_connection(&self.database_path)...
let (chats, handles, tapbacks, chatroom_participants) = self.build_caches(&db)?;
```
To:
```rust
let db = Database::open(&self.database_path)?;
let chats = db.load_chats()?;
let handles = db.load_handles()?;
let tapback_map = db.load_tapbacks()?;
```

- [ ] **Step 5: Rewrite export_chat to use paginated queries**

Replace `Message::stream(db, |msg_result| {...})` with:

```rust
let messages = db.messages_for_chat(
    &chat.guid,
    self.start_timestamp,
    self.end_timestamp,
)?;

for mut msg in messages {
    // ... convert and write each message
}
```

The date filtering moves from inside the stream callback into the query params (`after`/`before`). Note: `imessage-db` dates are already Unix ms, so `start_timestamp`/`end_timestamp` need adjustment — they should be stored as Unix ms instead of Cocoa nanos.

- [ ] **Step 6: Rewrite build_caches to use Database methods**

The `build_caches` method simplifies dramatically. Remove the `ChatToHandle::cache()` call — participant lookups use `db.chat_participants(chat_rowid)` directly.

```rust
fn build_caches(
    &self,
    db: &Database,
) -> Result<(
    HashMap<i64, Chat>,
    HashMap<i64, Handle>,
    TapbackResolver,
)> {
    let chats = db.load_chats()?;
    let handles = db.load_handles()?;

    let tapback_map = db.load_tapbacks()?;
    let tapbacks = TapbackResolver::from_map(tapback_map);

    Ok((chats, handles, tapbacks))
}
```

- [ ] **Step 7: Update all rowid types from i32 to i64**

`imessage-db` uses `i64` for rowids. Update all `chat_id: i32`, `handle_id: i32` etc. to `i64` throughout the codebase. This affects:
- `exporter.rs` — function signatures, HashMap keys
- `serialization/message.rs` — `MessageMetadata.rowid`, `MessageMetadata.chat_id`
- `serialization/chat.rs` — `SerializableChatContext.chat_id`
- `serialization/attachments.rs` — if using rowid
- `serialization/participant.rs` — `handle_id`
- `resolvers/tapbacks.rs` — if using rowids
- `contacts.rs` — handle_id mappings

- [ ] **Step 8: Run `cargo check` in workspace**

Run: `cargo check --workspace 2>&1`

Expected: Compilation errors only in `build_content()` (BubbleComponent usage) and attachment handling — those are addressed in Task 3.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: replace imessage-database connection and query layer with imessage-db"
```

---

## Task 3: Replace BubbleComponent/generate_text with typedstream decoder

**Files:**
- Modify: `crates/imessage-ndjson-core/src/exporter.rs` (build_content rewrite)
- Modify: `crates/imessage-ndjson-core/src/serialization/content.rs` (if needed)

This is the most significant logic change. The old flow was:
1. `msg.generate_text(db)` — mutates message, populates `msg.components: Vec<BubbleComponent>`
2. Match on `BubbleComponent::Text`, `BubbleComponent::Attachment`, etc.

The new flow is:
1. `msg.text` — already populated (or None)
2. `imessage_core::typedstream::decode_attributed_body(&msg.attributed_body)` — for rich text with attributes
3. `imessage_core::typedstream::extract_text(&msg.attributed_body)` — fallback when `msg.text` is None
4. `msg.attachments` — already populated (eager-loaded)

- [ ] **Step 1: Rewrite build_content()**

The new `build_content` no longer takes a `db: &Connection` parameter since it doesn't query attachments separately — they're already on the message.

```rust
use imessage_core::typedstream;

fn build_content(
    &self,
    msg: &Message,
    chat_id: i64,
    attachment_manager: &mut Option<AttachmentManager>,
) -> Result<SerializableContent> {
    let mut components = Vec::new();

    // Extract text — use msg.text, falling back to typedstream decode
    let text = msg.text.clone().or_else(|| {
        msg.attributed_body
            .as_deref()
            .and_then(typedstream::extract_text)
    });

    // Build text component with attributes from typedstream
    if let Some(ref text_str) = text {
        let attributes = self.extract_text_attributes(msg);
        components.push(ContentComponent::Text {
            text: text_str.clone(),
            attributes,
        });
    }

    // Build attachment components from eager-loaded attachments
    for att in &msg.attachments {
        let serializable = self.build_attachment(att, chat_id, attachment_manager)?;
        components.push(ContentComponent::Attachment(serializable));
    }

    Ok(SerializableContent {
        text,
        subject: msg.subject.clone(),
        components,
    })
}
```

- [ ] **Step 2: Implement extract_text_attributes()**

Parse the typedstream `attributedBody` blob for text effects. The decoder returns JSON with `runs` containing attribute dictionaries. Map known attribute keys to `TextEffect` variants.

```rust
fn extract_text_attributes(&self, msg: &Message) -> Vec<TextAttribute> {
    let body = match msg.attributed_body.as_deref() {
        Some(data) => data,
        None => return Vec::new(),
    };

    let decoded = match typedstream::decode_attributed_body(body) {
        Some(val) => val,
        None => return Vec::new(),
    };

    // decoded is a JSON array of { "string": "...", "runs": [...] }
    let mut attributes = Vec::new();

    if let Some(items) = decoded.as_array() {
        for item in items {
            if let Some(runs) = item.get("runs").and_then(|r| r.as_array()) {
                for run in runs {
                    let range = run.get("range").and_then(|r| r.as_array());
                    let attrs = run.get("attributes");

                    if let (Some(range), Some(attrs)) = (range, attrs) {
                        let start = range.first().and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        let length = range.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                        let effects = self.parse_effects(attrs);
                        if !effects.is_empty() {
                            attributes.push(TextAttribute {
                                start,
                                end: start + length,
                                effects,
                            });
                        }
                    }
                }
            }
        }
    }

    attributes
}

fn parse_effects(&self, attrs: &serde_json::Value) -> Vec<TextEffect> {
    let mut effects = Vec::new();

    if let Some(obj) = attrs.as_object() {
        for (key, value) in obj {
            match key.as_str() {
                "__kIMMentionAttributeName" => {
                    if let Some(id) = value.as_str() {
                        effects.push(TextEffect::Mention {
                            identifier: id.to_string(),
                        });
                    }
                }
                "__kIMDataDetectedAttributeName" => {
                    effects.push(TextEffect::OTP);
                }
                "__kIMLinkAttributeName" => {
                    if let Some(url) = value.as_str() {
                        effects.push(TextEffect::Link {
                            url: url.to_string(),
                        });
                    }
                }
                // Skip the part attribute — it's structural, not an effect
                "__kIMMessagePartAttributeName" => {}
                _ => {}
            }
        }
    }

    effects
}
```

**Note:** The exact attribute key names need verification against real `attributedBody` blobs. The keys above are reasonable guesses based on Apple's internal naming patterns. Capture unknown keys during development to discover the actual key names — add a `tracing::debug!` for unknown keys during initial testing.

- [ ] **Step 3: Rewrite build_attachment to use imessage-db's Attachment struct**

The `imessage-db` `Attachment` struct has slightly different field names. Update the mapping:

```rust
fn build_attachment(
    &self,
    att: &Attachment,
    chat_id: i64,
    attachment_manager: &mut Option<AttachmentManager>,
) -> Result<SerializableAttachment> {
    let mut converted_mime_type: Option<String> = None;

    // Resolve file path (imessage-db stores with ~/ prefix)
    let resolved_path = att.filename.as_ref().map(|f| {
        if f.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_default();
            f.replacen("~/", &format!("{}/", home), 1)
        } else {
            f.clone()
        }
    });

    let original_path = resolved_path.clone();

    // Handle copy/embed modes using attachment_manager
    // (same logic as before, but using resolved_path instead of att.path())
    let (copied_path, copy_error, embedded_data, embedded_encoding,
         embedded_compression, content_hash) = if self.embed_attachments {
        // ... embed logic (same as current, but pass resolved_path)
        // ...
        (None, None, None, None, None, None) // placeholder — preserve existing logic
    } else if self.copy_attachments {
        // ... copy logic
        (None, None, None, None, None, None) // placeholder — preserve existing logic
    } else {
        (None, None, None, None, None, None)
    };

    let final_mime_type = converted_mime_type.or_else(|| att.mime_type.clone());

    // Dimensions from attribution_info plist blob
    let dimensions = att.attribution_info.as_ref().and_then(|info| {
        let plist_val: plist::Value = plist::from_bytes(info).ok()?;
        let dict = plist_val.as_dictionary()?;
        let width = dict.get("pgensh")?.as_unsigned_integer()? as u32;
        let height = dict.get("pgensw")?.as_unsigned_integer()? as u32;
        Some(AttachmentDimensions { width, height })
    });

    Ok(SerializableAttachment {
        guid: att.guid.clone(),
        filename: att.filename.clone(),
        transfer_name: att.transfer_name.clone(),
        mime_type: final_mime_type,
        uti: att.uti.clone(),
        size_bytes: Some(att.total_bytes),
        transcription: None, // imessage-db doesn't have this field directly
        dimensions,
        is_sticker: att.is_sticker.unwrap_or(false),
        sticker_metadata: None,
        original_path,
        copied_path,
        copy_error: copy_error.map(|e| format!("{}", e)),
        embedded_data,
        embedded_encoding,
        embedded_compression,
        content_hash,
    })
}
```

- [ ] **Step 4: Update attachment_manager.rs to work without imessage-database's Attachment**

The `AttachmentManager` currently imports `imessage_database::tables::attachment::Attachment` and `imessage_database::util::platform::Platform`. Replace:

- `Attachment` references: Accept a resolved file path (`&Path`) and metadata instead of the full Attachment struct. Or create a thin `AttachmentInfo` struct that the exporter populates from `imessage-db`'s Attachment.
- `Platform::macOS`: Remove — the attachment manager doesn't need platform detection from the DB library. If it was used for path resolution, handle that in `build_attachment()`.

- [ ] **Step 5: Run `cargo check` in workspace**

Run: `cargo check --workspace 2>&1`

Expected: Clean compilation. All `imessage_database` imports should be gone.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: replace BubbleComponent/generate_text with imessage-core typedstream decoder"
```

---

## Task 4: Replace date handling and update timestamp types

**Files:**
- Modify: `crates/imessage-ndjson-core/src/exporter.rs`
- Modify: `crates/imessage-ndjson-core/src/serialization/message.rs`

`imessage-db` pre-converts dates to Unix milliseconds. Remove all Cocoa epoch conversion logic.

- [ ] **Step 1: Remove format_timestamp() and Cocoa epoch constants**

Delete the `format_timestamp()` function and the `APPLE_EPOCH` constant from `exporter.rs`. Dates from `imessage-db` are already Unix ms `Option<i64>` — None means "not set".

- [ ] **Step 2: Update date range parsing**

The `NdjsonExporter::new()` currently parses CLI date strings into Cocoa epoch nanoseconds via `parse_date_to_cocoa_nanos()`. Change to Unix milliseconds:

```rust
use imessage_core::dates::unix_ms_to_apple;

// Parse start date to Unix ms
let start_timestamp = start_date
    .map(|s| {
        let date = NaiveDate::parse_from_str(&s, "%Y-%m-%d")?;
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        Ok::<i64, anyhow::Error>(dt.and_utc().timestamp_millis())
    })
    .transpose()?;

// Parse end date to Unix ms (exclusive — add 1 day)
let end_timestamp = end_date
    .map(|s| {
        let date = NaiveDate::parse_from_str(&s, "%Y-%m-%d")?;
        let next_day = date + chrono::Duration::days(1);
        let dt = next_day.and_hms_opt(0, 0, 0).unwrap();
        Ok::<i64, anyhow::Error>(dt.and_utc().timestamp_millis())
    })
    .transpose()?;
```

**Note:** The `after`/`before` params on `MessageQueryParams` expect Unix ms, and `imessage-db` internally converts to Apple timestamps for the SQL query using `unix_ms_to_apple()`. So you pass Unix ms and it handles the rest.

- [ ] **Step 3: Update MessageMetadata serialization**

The `date`, `date_read`, `date_delivered`, `date_edited` fields in `MessageMetadata` should now be `Option<i64>` (Unix ms) directly from the entity, no conversion needed.

- [ ] **Step 4: Run `cargo check --workspace` and `cargo test --workspace`**

Expected: Clean build. Test any date-dependent assertions.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: remove Cocoa epoch conversion, dates are now Unix ms from imessage-db"
```

---

## Task 5: Update tapback resolver for string-based reaction types

**Files:**
- Modify: `crates/imessage-ndjson-core/src/resolvers/tapbacks.rs`
- Modify: `crates/imessage-ndjson-core/src/serialization/relationships.rs`

`imessage-db` pre-maps reaction types to strings (`"love"`, `"like"`, `"laugh"`, etc.) instead of integer enums. Simplify the resolver.

- [ ] **Step 1: Update TapbackResolver to use imessage-db Message**

Replace `use imessage_database::tables::messages::Message` with `use imessage_db::imessage::entities::Message`.

The `add_tapback` method and internal storage stay the same structurally — it's still a `HashMap<String, Vec<Message>>` keyed by `associated_message_guid`.

- [ ] **Step 2: Update tapback serialization**

In the code that converts tapbacks to `SerializableTapback`, the `tapback_type` field can now be directly set from `msg.associated_message_type` (already a `Option<String>` like `"love"`, `"like"`). No integer-to-string mapping needed.

```rust
SerializableTapback {
    tapback_type: msg.associated_message_type.clone().unwrap_or_default(),
    emoji: msg.associated_message_emoji.clone(),
    // ... rest
}
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: simplify tapback resolver with string-based reaction types"
```

---

## Task 6: Update contacts.rs and phone normalization

**Files:**
- Modify: `crates/imessage-ndjson-core/src/contacts.rs`
- Modify: `crates/imessage-ndjson-core/Cargo.toml`

The `contacts.rs` module uses `imessage_database` for three things: `TableError`, `get_connection`, and `home()`. Replace all three.

- [ ] **Step 1: Replace imessage-database imports in contacts.rs**

Old:
```rust
use imessage_database::{error::table::TableError, tables::table::get_connection, util::dirs::home};
```

New:
```rust
use rusqlite::Connection;
```

- Replace `get_connection()` calls with `Connection::open_with_flags()` using `SQLITE_OPEN_READ_ONLY`.
- Replace `home()` with `std::env::var("HOME")` or the `home` crate (already in imessage-core's deps).
- Replace `TableError` with `anyhow::Error` or a local error type.

- [ ] **Step 2: Consider adopting imessage-core phone normalization**

The current `contacts.rs` has manual `phone_keys()` generating multiple lookup variants. `imessage-core` offers `phone::normalize_address()` using the `phonenumber` crate (Google's libphonenumber).

This is optional — the current approach works. If adopted, add `phonenumber = "0.3"` to deps and simplify the normalization. This can also be done as a follow-up.

- [ ] **Step 3: Run `cargo check --workspace`**

Expected: No `imessage_database` imports remain anywhere.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: remove imessage-database from contacts module"
```

---

## Task 7: Remove imessage-database dependency entirely

**Files:**
- Modify: `crates/imessage-ndjson-core/Cargo.toml`

- [ ] **Step 1: Remove imessage-database from Cargo.toml**

Delete the `imessage-database = "3.2.1"` line. Update `rusqlite` version to `0.32` (matching `imessage-db`).

- [ ] **Step 2: Verify no remaining imports**

```bash
grep -r "imessage_database" crates/imessage-ndjson-core/src/
```

Expected: No results.

- [ ] **Step 3: Run full build and test**

```bash
cargo build --workspace
cargo test --workspace
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: remove GPL imessage-database dependency, now fully MIT"
```

---

## Task 8: Rewrite CLI binary with librebar

**Files:**
- Modify: `src/main.rs` (full rewrite)
- Modify: `src/cli.rs` (add librebar CommonArgs)
- Modify: `Cargo.toml` (root — already has librebar dep from Task 1)

- [ ] **Step 1: Update cli.rs to flatten librebar CommonArgs**

```rust
use clap::Parser;
use librebar::cli::CommonArgs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "imessage-ndjson-exporter",
    about = "Export iMessage data to NDJSON format",
    version,
    after_help = "Examples:\n  imessage-ndjson-exporter --output ./export\n  imessage-ndjson-exporter --output ./export --copy-attachments --convert-attachments"
)]
pub struct Cli {
    /// Common flags (--verbose, --quiet, --color, --chdir)
    #[command(flatten)]
    pub common: CommonArgs,

    // ... all existing domain-specific fields stay the same:
    // database_path, output_dir, start_date, end_date, etc.
    // (copy them from current cli.rs, removing any that overlap with CommonArgs)
}
```

- [ ] **Step 2: Rewrite main.rs with librebar bootstrap**

```rust
use anyhow::{Context, Result};
use clap::Parser;
use imessage_ndjson_core::{NdjsonExporter, attachment_manager::CompressionMode};

mod cli;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize librebar app (logging, crash handler)
    let _app = librebar::init("imessage-ndjson-exporter")
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_cli(cli.common.clone())
        .logging()
        .crash_handler()
        .start()
        .map_err(|e| anyhow::anyhow!("Failed to initialize: {}", e))?;

    // Resolve database path
    let db_path = cli
        .database_path
        .clone()
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(format!("{}/Library/Messages/chat.db", home))
        });

    if !db_path.exists() {
        anyhow::bail!("Database not found: {:?}", db_path);
    }

    // Create output directory
    std::fs::create_dir_all(&cli.output_dir)
        .context("Failed to create output directory")?;

    // Parse compression mode
    let embed_compression = CompressionMode::parse(&cli.embed_compression)
        .ok_or_else(|| anyhow::anyhow!("Invalid compression mode: {}", cli.embed_compression))?;

    // Create and run exporter
    let exporter = NdjsonExporter::new(
        &db_path,
        &cli.output_dir,
        cli.custom_name.clone(),
        !cli.common.quiet,
        cli.conversation_filter.clone(),
        cli.contacts_path.clone(),
        cli.copy_attachments,
        cli.convert_attachments,
        cli.attachments_dir.clone(),
        cli.embed_attachments,
        cli.max_embed_size,
        embed_compression,
        cli.include_avatars,
        cli.start_date.clone(),
        cli.end_date.clone(),
    )?;

    exporter.export()
}
```

- [ ] **Step 3: Update root Cargo.toml if needed**

Ensure `librebar` path dep points to `../librebar` and has the right features:

```toml
librebar = { version = "0.1", features = ["cli", "logging", "crash"] }
```

- [ ] **Step 4: Test CLI**

```bash
cargo run -- --help
cargo run -- --output /tmp/test-export
```

Expected: Help text shows librebar common flags (`-v`, `-q`, `--color`) plus all domain flags. Export runs normally.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: rewrite CLI binary with librebar bootstrap"
```

---

## Task 9: Update Justfile and CI

**Files:**
- Modify: `Justfile`

- [ ] **Step 1: Update Justfile for workspace**

Ensure all commands work in workspace context:

```just
build:
  cargo build --workspace --release

clippy:
  cargo +{{toolchain}} clippy --workspace --all-targets --all-features --message-format=short -- -D warnings

test:
  cargo nextest run --workspace --all-features

fmt:
  cargo fmt --all -- --config-path .config/rustfmt.toml
```

- [ ] **Step 2: Run full check suite**

```bash
just check
```

Expected: fmt, clippy, deny, test, doc-test all pass.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore: update Justfile for workspace layout"
```

---

## Task 10: Final verification and cleanup

- [ ] **Step 1: Verify no GPL dependencies remain**

```bash
cargo tree --workspace --format "{p} {l}" | grep -i gpl
```

Expected: No results.

- [ ] **Step 2: Verify library crate is usable standalone**

```bash
cd crates/imessage-ndjson-core
cargo check
cargo test --all-features
```

- [ ] **Step 3: Verify binary works end-to-end**

```bash
cargo run --release -- --output /tmp/ndjson-test
```

Expected: Successful export.

- [ ] **Step 4: Update root LICENSE to MIT**

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: finalize MIT migration and workspace split"
```

---

## Migration Risk Notes

1. **rusqlite version gap:** `imessage-db` uses 0.32, you were on 0.37. The bundled feature should prevent ABI issues, but watch for API differences in `Connection` methods used by `contacts.rs`.

2. **Typedstream attribute keys:** The exact NSAttributedString attribute key names (e.g., `__kIMMentionAttributeName`) need verification against real data. Add debug logging to capture unknown keys during initial testing.

3. **Date precision:** `imessage-db` returns Unix ms; your current code uses Cocoa epoch nanos. Verify no precision loss matters for your consumers.

4. **Attachment path resolution:** `imessage-db` stores paths with `~/` prefix. Your current code uses `att.path()` which resolves tildes. You'll need to handle tilde expansion yourself.

5. **No streaming:** The pagination approach loads more into memory per page than the old streaming callbacks. For very large databases, tune the page size. The 500-message page size is a reasonable starting point.

6. **imessage-db 0.1.0 freshness:** If you hit bugs, you can patch locally and contribute fixes upstream. The crate is small enough to vendor if needed.
