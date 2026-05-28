# iMessage NDJSON Exporter

A Rust toolkit for exporting iMessage conversations to **NDJSON** (newline-delimited JSON) format, providing a lossless, structured export of all message data.

## Overview

This project exports iMessage data from the macOS `chat.db` database to NDJSON format, capturing:

- **Complete message metadata** - timestamps, sender, service type, read status
- **Rich text formatting** - mentions, links, styles, animations
- **Attachments** - files, images, videos with full metadata
- **Reactions/tapbacks** - who reacted, when, to which message part
- **Message edits** - edit history with timestamps
- **Threaded replies** - reply relationships and thread structure
- **Group actions** - participant changes, name updates, icons
- **Stickers** - including Genmoji prompts and animated sticker conversion

## Project Structure

This is a Cargo workspace with two crates:

```
crates/
  imessage-ndjson-core/       MIT library — embeddable in other tools
  imessage-ndjson-exporter/   CLI binary — thin front using librebar + clap
```

**`imessage-ndjson-core`** is the reusable library containing all export logic, serialization, attachment handling, contact resolution, and format converters. It can be embedded in other Rust applications as a dependency.

**`imessage-ndjson-exporter`** is a thin CLI binary that wires up the library with [librebar](https://crates.io/crates/librebar) for structured logging and crash handling, and [clap](https://crates.io/crates/clap) for argument parsing.

### Key Dependencies

- **[imessage-db](https://crates.io/crates/imessage-db)** - Read-only SQLite access layer for macOS `chat.db` (MIT)
- **[imessage-core](https://crates.io/crates/imessage-core)** - Typedstream decoder for `attributedBody` blobs, date conversion (MIT)
- **[librebar](https://crates.io/crates/librebar)** - CLI application framework with structured logging and crash handling (MIT)
- **serde/serde_json** - JSON serialization
- **clap** - Command-line argument parsing
- **indicatif** - Progress indicators

## Installation

```bash
# Clone the repository
git clone https://github.com/claylo/imessage-ndjson-exporter.git
cd imessage-ndjson-exporter

# Build the release binary
cargo build --workspace --release

# The binary will be at target/release/imessage-ndjson-exporter
```

### Requirements

- macOS Sequoia (15) or later
- Full Disk Access permission (for `~/Library/Messages/chat.db`)
- Rust toolchain 1.85+

## Usage

### Basic Export

Export all conversations to a directory:

```bash
imessage-ndjson-exporter --output ./export
```

The tool auto-detects the iMessage database at `~/Library/Messages/chat.db`.

### Custom Database Path

```bash
imessage-ndjson-exporter \
  --database ~/Library/Messages/chat.db \
  --output ./export
```

### Date Range Filter

Export messages within a specific date range:

```bash
imessage-ndjson-exporter \
  --output ./export \
  --start-date 2024-01-01 \
  --end-date 2024-12-31
```

### Filter by Contact

Export only conversations with specific contacts:

```bash
imessage-ndjson-exporter \
  --output ./export \
  -t "steve@apple.com,Jane Doe,5558675309"
```

The filter accepts:
- **Contact names** (substring match): `"Jane Doe"`, `"Steve"`
- **Phone numbers** (normalized): `"5558675309"`, `"+15558675309"`
- **Email addresses** (case-insensitive): `"steve@apple.com"`

Multiple filters can be comma-separated. All conversations with **any** matching participant will be exported, including group conversations.

**Note:** The `-t` flag requires access to the macOS Contacts database. Use `--contacts-path` to specify a custom location if needed.

### With Custom Name

Override the default "Me" name for messages you sent:

```bash
imessage-ndjson-exporter \
  --output ./export \
  --custom-name "Your Name"
```

### Attachment Handling

Three modes for attachments:

1. **Reference in-place** (default) - Include original file paths in JSON without copying
2. **Copy** (`--copy-attachments`) - Copy files to export directory with optional format conversion
3. **Embed** (`--embed-attachments`) - Embed files as base64 in JSON

#### Copy Attachments

```bash
imessage-ndjson-exporter \
  --output ./export \
  --copy-attachments
```

Files are named using SHA256 content hashes for deduplication:

```
export/
  attachments/
    chat_123/
      a3f2c8d9e4b1f7a2.jpg
  chat_123.ndjson
```

#### Embed Attachments

```bash
imessage-ndjson-exporter \
  --output ./export \
  --embed-attachments \
  --embed-compression auto \
  --max-embed-size 10485760
```

Compression options: `auto` (default), `zstd`, `gzip`, `none`. Mutually exclusive with `--copy-attachments`.

#### Convert Attachments

Convert Apple-specific formats to widely-compatible formats:

```bash
imessage-ndjson-exporter \
  --output ./export \
  --copy-attachments \
  --convert-attachments
```

Conversions:
- **HEIC → JPEG** (photos) via `sips` or `imagemagick`
- **Sticker HEIC → PNG** (static stickers) via `sips` or `imagemagick`
- **Sticker HEICS → GIF** (animated stickers) via `ffmpeg`
- **MOV → MP4** (videos) via `ffmpeg`
- **CAF → M4A** (audio) via `afconvert` or `ffmpeg`

Required tools: `brew install ffmpeg imagemagick`

### Include Participant Avatars

```bash
imessage-ndjson-exporter \
  --output ./export \
  --include-avatars
```

Creates `chat_XX_participants.ndjson` files and copies avatar images to `avatars/` with content-based deduplication.

### Common Flags

These flags are provided by librebar and available on all commands:

| Flag | Description |
|------|-------------|
| `-v` | Increase verbosity (repeatable: `-v` = debug, `-vv` = trace) |
| `-q` | Quiet mode — suppress progress indicators and success messages |
| `--color auto\|always\|never` | Control color output |
| `-C <dir>` | Change working directory before running |

## Output Format

Each `.ndjson` file contains one JSON object per line. Each message is a self-contained record.

### Example Message

```json
{
  "message_type": "normal",
  "metadata": {
    "rowid": 12345,
    "guid": "p:0/1234ABCD-5678-90EF-GHIJ-KLMNOPQRSTUV",
    "date": 1732723800000,
    "date_read": 1732723875000,
    "date_delivered": 1732723802000,
    "date_edited": null,
    "service": "iMessage",
    "is_from_me": false,
    "is_read": true,
    "chat_id": 42,
    "is_deleted": false
  },
  "sender": {
    "handle_id": 7,
    "identifier": "+15551234567",
    "contact_name": "Jane Doe"
  },
  "chat_context": {
    "chat_id": 42,
    "chat_identifier": "chat123456",
    "display_name": "Project Team",
    "service_name": "iMessage",
    "participants": ["chat123456"]
  },
  "content": {
    "text": "Hey, check this out!",
    "subject": null,
    "components": [
      {
        "type": "text",
        "text": "Hey, check this out!",
        "attributes": []
      }
    ]
  },
  "relationships": {
    "thread_originator_guid": null,
    "thread_originator_part": null,
    "num_replies": 0,
    "tapbacks": [],
    "edit_history": null
  },
  "expressive_effect": null,
  "group_action": null
}
```

**Note:** Timestamps are Unix milliseconds. Dates from `imessage-db` are pre-converted from Apple's Cocoa epoch.

### Schema Documentation

See [SCHEMA.md](SCHEMA.md) for complete schema documentation.

## Processing NDJSON Files

### Using `jq`

```bash
# Messages from you
cat chat_42.ndjson | jq -c 'select(.metadata.is_from_me == true)'

# Message counts by sender
cat chat_42.ndjson | jq -r '.sender.contact_name // .sender.identifier' | sort | uniq -c

# Messages with attachments
cat chat_42.ndjson | jq -c 'select(.content.components[] | .type == "attachment")'

# All text content
cat chat_42.ndjson | jq -r '.content.text // empty'
```

### Using Python

```python
import json

with open('chat_42.ndjson', 'r') as f:
    for line in f:
        msg = json.loads(line)
        if msg['metadata']['is_from_me']:
            print(f"{msg['metadata']['date']}: {msg['content']['text']}")
```

## Using the Library

To embed `imessage-ndjson-core` in your own tool:

```toml
[dependencies]
imessage-ndjson-core = { git = "https://github.com/claylo/imessage-ndjson-exporter.git" }
```

```rust
use imessage_ndjson_core::NdjsonExporter;
use imessage_ndjson_core::attachment_manager::CompressionMode;

let exporter = NdjsonExporter::new(
    &db_path,
    &output_dir,
    None,           // custom_name
    true,           // show_progress
    None,           // conversation_filter
    None,           // contacts_path
    false,          // copy_attachments
    false,          // convert_attachments
    "attachments".to_string(),
    false,          // embed_attachments
    10_485_760,     // max_embed_size
    CompressionMode::Auto,
    false,          // include_avatars
    None,           // start_date
    None,           // end_date
)?;

exporter.export()?;
```

## License

MIT

## Credits

Database access powered by [imessage-rs](https://github.com/jesec/imessage-rs) by jesec — MIT-licensed crates for iMessage database access and typedstream decoding.

Test fixtures in `test_data/` (plist samples, typedstream blobs, sticker assets) and the `examples/` NDJSON files derived from them originate from the [imessage-exporter](https://github.com/ReagentX/imessage-exporter) project by Christopher Sardegna (GPL-3.0). These files are test and documentation artifacts only — they are not compiled into any binary or library.

## Contributing

Issues and pull requests welcome.
