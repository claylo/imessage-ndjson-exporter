# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust command-line tool that exports iMessage conversations from macOS's `chat.db` to NDJSON (newline-delimited JSON) format. It provides a lossless, structured export of all message data including metadata, reactions, edits, attachments, and special features.

The tool is built as a standalone binary that depends on `imessage-database` from the [imessage-exporter](https://github.com/ReagentX/imessage-exporter) project as a library dependency (via path dependency).

## Building and Running

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized, stripped binary)
cargo build --release

# Run in development mode
cargo run -- --output ./export

# Run release binary
./target/release/imessage-ndjson-exporter --output ./export
```

### Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture
```

### Code Quality

```bash
# Check for compilation errors without building
cargo check

# Run clippy linter
cargo clippy

# Format code
cargo fmt

# Check if code is formatted
cargo fmt -- --check
```

## Architecture

### Module Structure

The codebase is organized into four main modules:

1. **`exporter`** (`src/exporter.rs`) - Core export logic
   - `NdjsonExporter` orchestrates the entire export process
   - Builds caches for chats, handles, and tapbacks from the database
   - Streams messages per chat and writes NDJSON files
   - Converts imessage-database types to serializable structs

2. **`serialization`** (`src/serialization/`) - Data transformation layer
   - Converts imessage-database types into serializable structs
   - Modules: `message`, `chat`, `content`, `relationships`, `attachments`
   - All structs use serde's `#[derive(Serialize)]` for JSON output
   - Design pattern: Keep imessage-database types internal, expose clean serializable structures

3. **`resolvers`** (`src/resolvers/`) - Lookup and resolution utilities
   - `ContactResolver` - Maps handles to contact names (supports custom "Me" name)
   - `TapbackResolver` - Maintains cache of reactions (tapbacks) by message GUID
   - `ReplyResolver` - Placeholder for thread reply resolution (not yet implemented)

4. **`cli`** (`src/cli.rs`) - Command-line interface
   - Uses clap 4.x with derive macros
   - Defines all CLI arguments and options

### Key Dependencies

- **imessage-database** - Database parsing library
- **rusqlite** - SQLite database access (version pinned to match imessage-database: `=0.37.0`)
- **serde/serde_json** - JSON serialization
- **clap** - CLI argument parsing
- **indicatif** - Progress bars and spinners
- **chrono** - Date/time formatting

### Data Flow

1. **Cache Building Phase** - Load all chats, handles, and tapbacks into memory
2. **Per-Chat Export** - For each chat:
   - Stream messages from database (avoids loading all into memory)
   - Call `msg.generate_text(db)` to populate message text and components
   - Convert each message to `SerializableMessage` using resolvers
   - Write as single-line JSON to `.ndjson` file
3. **File Output** - One `chat_{id}.ndjson` file per conversation

### Important Patterns

**Timestamp Conversion**: iMessage uses Apple's Cocoa epoch (2001-01-01), which is 978307200 seconds after Unix epoch. The `format_timestamp()` function handles this conversion (see `exporter.rs:397-411`).

**Streaming Architecture**: Messages are streamed using `Message::stream(db, |msg_result| {...})` rather than loading all into memory. This is critical for handling large message databases.

**Component-Based Content**: Messages use a component model (`BubbleComponent`) that can contain text, attachments, apps, or retracted content. Text components have attributes (ranges) with effects (mentions, links, OTP codes, etc.).

**Caching Strategy**: Chats, handles, and tapbacks are cached upfront because they're frequently accessed. Messages are streamed per-chat to minimize memory usage.

## Development Notes

### Path Dependency

This project depends on `imessage-database` as a path dependency pointing to `../imessage-exporter/imessage-database`. Both repositories should be cloned side-by-side:

```
parent-directory/
  imessage-ndjson-exporter/
  imessage-exporter/
    imessage-database/
```

### Incomplete Features (TODOs)

Several features are stubbed but not fully implemented:

- Tapback resolution (relationships.rs:278)
- Edit history tracking (relationships.rs:279)
- Expressive effects (message.rs:288)
- Group actions (message.rs:289)
- Attachment metadata serialization (exporter.rs:377-379)
- App message serialization (exporter.rs:380-382)

When implementing these, refer to the corresponding types in imessage-database and add serialization modules in `src/serialization/`.

### Conversation Filter

The `-t, --conversation-filter` flag enables filtering by contact names, phone numbers, or emails. Implementation details:

**ContactsIndex (`src/contacts.rs`):** Loads macOS contacts database from `~/Library/Application Support/AddressBook/Sources/*/AddressBook-v22.abcddb`. Builds `HashMap<String, Name>` mapping normalized phone/email to contact info. Supports both macOS (`AddressBook-v22.abcddb`) and iOS (`AddressBook.sqlitedb`) formats.

**Phone Normalization:** Generates multiple lookup keys for US numbers (+1 prefix variants), international numbers, etc. See `phone_keys()` in contacts.rs. For example, `+12345678901` generates keys: `12345678901`, `+12345678901`, `2345678901`, `+2345678901`.

**Email Normalization:** Lowercase, removes angle brackets. See `normalize_email()` in contacts.rs. Case-insensitive matching.

**Filtering Process (`exporter.rs:resolve_filtered_chats`):**
1. Parse comma-separated filter string into terms
2. Build participants map (handle_id -> handle details) from iMessage database
3. Use `ContactsIndex.build_participants_map()` to resolve handles to Names
4. For each filter term, find matching Names (substring match on first/last/full/details fields)
5. Collect all handle_ids from matching Names
6. Query `chat_handle_join` table to build chatroom participants map
7. Find chats containing any of the selected handles (set intersection)
8. Check additional 1:1 chats that may not be in chat_handle_join
9. Return selected chat_ids

**Integration (`exporter.rs:export`):**
- Build ContactsIndex if `-t` flag is specified (fails with error if contacts DB unavailable)
- Call `resolve_filtered_chats()` to get selected chat IDs
- Fail if filter matches no conversations
- Skip non-matching chats during export iteration
- ContactResolver now uses ContactsIndex for contact name lookup during export

**Error Handling:** If `-t` is used but contacts database unavailable, fails with clear error message: "Failed to load contacts database (required for -t filter). Use --contacts-path to specify custom location or remove -t flag."

### Contact Resolution

The `ContactResolver` (`src/resolvers/contacts.rs`) uses `ContactsIndex` for resolving handle identifiers to contact names. It caches lookup results for performance and returns the custom name for messages sent by the database owner.

### Database Access

Always use the imessage-database library's abstractions (`Table` trait, `stream()` methods) rather than writing raw SQL. This ensures compatibility across different iOS/macOS versions.

### Error Handling

The codebase uses `anyhow::Result` for error propagation. When message text generation fails (common with corrupted database entries), it logs a warning but continues processing.
