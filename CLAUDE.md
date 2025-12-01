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

The codebase is organized into five main modules:

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

4. **`converters`** (`src/converters/`) - Attachment format conversion
   - `models` - Converter trait and tool detection
   - `image` - HEIC → JPEG conversion (sips or imagemagick)
   - `video` - MOV → MP4 conversion (ffmpeg, software-only)
   - `audio` - CAF → M4A conversion (afconvert or ffmpeg)
   - `common` - Shared utilities (run_command, ensure_paths)

5. **`cli`** (`src/cli.rs`) - Command-line interface
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

### Attachment Handling Modes

The exporter supports three mutually-exclusive modes for handling attachments:

**1. Reference In-Place (Default)**

When neither `--copy-attachments` nor `--embed-attachments` is specified, attachments are referenced by their original absolute paths without copying or embedding. The `original_path` field in the JSON contains the full path to the attachment file.

- Fastest export (no file I/O for attachments)
- Minimal disk space usage
- Preserves original file structure
- Ideal for local analysis and archival

**2. Copy Mode (`--copy-attachments`)**

Copies attachment files to the export directory in an organized structure: `attachments/chat_ID/hash.ext`. Files are deduplicated using SHA256 content hashing. The `copied_path` field contains the relative path from the output directory.

- Portable exports (all files in one directory tree)
- Supports format conversion (`--convert-attachments`)
- Deduplicates identical files across chats
- Organizes files by chat for easy navigation

**3. Embed Mode (`--embed-attachments`)**

Embeds attachment data directly in the JSON as base64-encoded strings. The `embedded_data` field contains the encoded data, along with `embedded_encoding` and `embedded_compression` metadata.

- Fully self-contained exports (single NDJSON file per chat)
- No external file dependencies
- Significantly increases JSON file size
- Supports compression (auto, gzip, zstd, none)
- Size limit configurable via `--max-embed-size`

**Implementation Notes:**

The exporter checks flags in order: `embed_attachments` → `copy_attachments` → default (reference in-place). The `AttachmentManager` is only created when copying or embedding is enabled. In reference mode, `attachment.path()` is called to get the original path, which is included in the serialized output.

### Attachment Conversion

The `--convert-attachments` flag enables format conversion using external tools (requires `--copy-attachments`):

**Implementation:** `src/converters/`
- `models.rs` - Converter trait and tool detection
- `image.rs` - HEIC → JPEG conversion for photos (sips or imagemagick)
- `sticker.rs` - Sticker-specific conversions (HEIC → PNG, HEICS → GIF)
- `video.rs` - MOV → MP4 conversion (ffmpeg, software-only)
- `audio.rs` - CAF → M4A conversion (afconvert or ffmpeg)

**Tool Detection:** Runs at startup using `which` (Unix) or `where` (Windows)

**Error Handling:** Fails fast if `--convert-attachments` is specified but required tools are missing. Provides installation instructions.

**Conversion Strategy:**
- **Photos (HEIC → JPEG):** Direct conversion using sips or imagemagick
- **Stickers (HEIC → PNG):** Detected via `attachment.is_sticker` field, converted to PNG to preserve transparency. Sticker HEIC files contain 5 image resolutions; only the highest (320x320) is extracted.
- **Animated Stickers (HEICS → GIF):** Complex multi-stage process using ffmpeg:
  1. Extract video frames from stream 2 (animation data)
  2. Extract alpha masks from stream 3 (transparency data)
  3. Merge frames with alpha masks to create transparent PNGs
  4. Generate transparency-aware color palette
  5. Create final animated GIF with proper transparency
  6. Clean up temporary files
- **Video (MOV → MP4):** Two-stage process:
  1. Try remuxing (container change only, fast, no re-encoding)
  2. Fall back to software re-encoding with libx264 if remuxing fails
- **Audio (CAF → M4A):** Convert to MP4 container with AAC audio

**Extension Tracking:** Conversion functions update the destination path's extension. The relative path returned to the serializer automatically reflects the new extension.

**MIME Type Updates:** When conversion occurs, the MIME type in the JSON output is updated to reflect the converted format (e.g., `image/heic` → `image/jpeg` for photos, `image/heic` → `image/png` for stickers, `image/heics` → `image/gif` for animated stickers).

**Progress Indication:** Video conversions show progress messages to prevent "frozen" appearance during slow re-encoding operations.

### Incomplete Features (TODOs)

Several features are stubbed but not fully implemented:

- Tapback resolution (relationships.rs:278)
- Edit history tracking (relationships.rs:279)
- Expressive effects (message.rs:288)
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

### Avatar Support

The `--include-avatars` flag enables extraction and export of participant avatars from the macOS Contacts database. Implementation details:

**AvatarManager (`src/avatar_manager.rs`):** Handles copying avatar image files from the contacts database to the output directory. Uses SHA256 hashing for content-based deduplication - multiple participants with the same avatar image will only store one copy. Returns relative paths in the format `avatars/<hash>.jpg`.

**Avatar Query (`src/contacts.rs:get_avatar_paths`):** Extends `ContactsIndex` with avatar support. Queries the `ZABCDIMAGEDATA` table to find avatar image paths for each contact. Maps phone numbers and email addresses to avatar file paths. Returns `HashMap<String, PathBuf>` mapping participant identifiers to source avatar paths.

**SQL Query:** Joins `ZABCDRECORD` (contacts), `ZABCDPHONENUMBER`/`ZABCDEMAILADDRESS` (contact details), and `ZABCDIMAGEDATA` (avatar metadata) tables. Avatar images are stored in `Images/` subdirectory relative to the contacts database.

**SerializableParticipant (`src/serialization/participant.rs`):** Structure representing a conversation participant with contact information and optional avatar path. Fields: `handle_id` (i32), `identifier` (String), `contact_name` (Option<String>), `avatar_path` (Option<String>). The `avatar_path` field is set to `null` if no avatar is available.

**Participants Files:** For each conversation, creates a `chat_XX_participants.ndjson` file containing one JSON object per line, one for each participant. Written after the main chat export completes.

**Integration (`exporter.rs`):**
1. Query avatar paths from contacts database before creating `ContactResolver` (ownership requirement)
2. Create `AvatarManager` if `--include-avatars` flag is specified
3. Cache `ChatToHandle` relationships to map chats to participant handle IDs
4. After exporting each chat's messages, call `write_participants_file()` to create the participants NDJSON file
5. For each participant, look up avatar source path and copy using `AvatarManager`

**Borrow Checker Note:** Avatar paths must be queried BEFORE creating `ContactResolver` because the resolver consumes the `ContactsIndex`. The avatar paths `HashMap` is then passed separately to the participant writing logic.

### Database Access

Always use the imessage-database library's abstractions (`Table` trait, `stream()` methods) rather than writing raw SQL. This ensures compatibility across different iOS/macOS versions.

### Error Handling

The codebase uses `anyhow::Result` for error propagation. When message text generation fails (common with corrupted database entries), it logs a warning but continues processing.
