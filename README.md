# iMessage NDJSON Exporter

A standalone tool to export iMessage conversations to **NDJSON** (newline-delimited JSON) format, providing a lossless, structured export of all message data.

## Overview

This tool exports iMessage data from the macOS iMessage database (`chat.db`) to NDJSON format, capturing:

- ✅ **Complete message metadata** - timestamps, sender, service type, read status
- ✅ **Rich text formatting** - mentions, links, styles, animations
- ✅ **Attachments** - files, images, videos with full metadata
- ✅ **Reactions/tapbacks** - who reacted, when, to which message part
- ✅ **Message edits** - complete edit history with timestamps
- ✅ **Threaded replies** - reply relationships and thread structure
- ✅ **App messages** - polls, locations, music, handwriting, etc.
- ✅ **Group actions** - participant changes, name updates, icons
- ✅ **Stickers** - including Genmoji prompts and Memoji metadata

## Features

### Lossless Export
Captures **all** message data available in the iMessage database, matching the completeness of the [imessage-exporter](https://github.com/ReagentX/imessage-exporter) HTML/text exporters.

### NDJSON Format
- One JSON object per line (NDJSON/JSON Lines format)
- Self-contained messages with full context
- Easy to process with standard tools (`jq`, `grep`, etc.)
- Streamable for large datasets

### One File Per Conversation
Follows the same pattern as imessage-exporter - each conversation gets its own `.ndjson` file for easy organization.

### Memory Efficient
Streams messages from the database without loading everything into memory.

## Installation

```bash
# Clone or navigate to the project directory
cd /path/to/imessage-ndjson-exporter

# Build the release binary
cargo build --release

# The binary will be at target/release/imessage-ndjson-exporter
```

## Usage

### Basic Export

Export all conversations to a directory:

```bash
./target/release/imessage-ndjson-exporter --output ./export
```

The tool will auto-detect the iMessage database location (`~/Library/Messages/chat.db`).

### Custom Database Path

```bash
./target/release/imessage-ndjson-exporter \
  --database ~/Library/Messages/chat.db \
  --output ./export
```

### With Custom Name

Override the default "Me" name for messages you sent:

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  --custom-name "Your Name"
```

### Filter by Contact

Export only conversations with specific contacts:

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  -t "steve@apple.com,Jane Doe,5558675309"
```

The filter accepts:
- **Contact names** (substring match, case-sensitive): `"Jane Doe"`, `"Steve"`
- **Phone numbers** (normalized, with/without country code): `"5558675309"`, `"+15558675309"`
- **Email addresses** (case-insensitive): `"steve@apple.com"`

Multiple filters can be comma-separated. All conversations with **any** matching participant will be exported, including group conversations.

**Note:** The `-t` flag requires access to the macOS Contacts database. By default, it scans `~/Library/Application Support/AddressBook/Sources/*/AddressBook-v22.abcddb`. If the contacts database is unavailable, the export will fail with a clear error message.

### Custom Contacts Database

If your contacts database is in a non-standard location:

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  -t "Jane Doe" \
  --contacts-path ~/path/to/AddressBook-v22.abcddb
```

### Attachment Handling

#### Copy Attachments to Directory

Copy attachment files to the output directory (organized by chat):

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  --copy-attachments
```

Files are named using SHA256 content hashes to prevent duplicates and enable deduplication. Original filenames are preserved in the JSON metadata. Directory structure:

```
export/
  attachments/
    chat_123/
      a3f2c8d9e4b1f7a2.jpg
      b7c4d1e8f2a9c3d5.heic
  chat_123.ndjson
```

#### Embed Attachments in JSON

Embed attachments directly in the JSON output (base64-encoded):

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  --embed-attachments
```

This makes exports fully portable (single file per chat) but significantly increases file size. **Mutually exclusive with `--copy-attachments`**.

**Compression options** (`--embed-compression`):
- `auto` (default) - Smart detection: skips compression for already-compressed formats (JPEG, MP4, HEIC, etc.), uses zstd for everything else
- `zstd` - Force zstd compression (fast, excellent compression ratios)
- `gzip` - Force gzip compression (broader compatibility)
- `none` - No compression (base64 only)

**Size limit** (`--max-embed-size`):
- Default: 10MB (10485760 bytes)
- Attachments larger than this will be skipped with an error in the JSON

Example with custom settings:

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  --embed-attachments \
  --max-embed-size 5242880 \
  --embed-compression zstd
```

### Quiet Mode

Disable progress indicators:

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  --no-progress
```

### Verbose Mode

Enable debug logging:

```bash
./target/release/imessage-ndjson-exporter \
  --output ./export \
  --verbose
```

## Output Format

Each `.ndjson` file contains one JSON object per line. Each message is a self-contained record with complete context.

### Example Message

```json
{
  "message_type": "normal",
  "metadata": {
    "rowid": 12345,
    "guid": "p:0/1234ABCD-5678-90EF-GHIJ-KLMNOPQRSTUV",
    "date": "2024-11-27T10:30:00-0800",
    "date_read": "2024-11-27T10:31:15-0800",
    "date_delivered": "2024-11-27T10:30:02-0800",
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
        "attributes": [
          {
            "start": 0,
            "end": 21,
            "effects": [{"type": "default"}]
          }
        ]
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

### Schema Documentation

See [SCHEMA.md](SCHEMA.md) for complete schema documentation.

## Processing NDJSON Files

### Using `jq`

Extract all messages from you:

```bash
cat chat_42.ndjson | jq -c 'select(.metadata.is_from_me == true)'
```

Get message counts by sender:

```bash
cat chat_42.ndjson | jq -r '.sender.contact_name // .sender.identifier' | sort | uniq -c
```

Find messages with attachments:

```bash
cat chat_42.ndjson | jq -c 'select(.content.components[] | .type == "attachment")'
```

Extract all text content:

```bash
cat chat_42.ndjson | jq -r '.content.text // empty'
```

### Extracting Embedded Attachments with `jq`

When using `--embed-attachments`, attachments are base64-encoded (and optionally compressed) within the JSON. Here's how to extract them:

#### Extract a Single Attachment (uncompressed or gzip)

```bash
# Find message with embedded attachment
cat chat_42.ndjson | jq -r '
  select(.content.components[]? | .type == "attachment" and .embedded_data != null) |
  .content.components[] |
  select(.type == "attachment" and .embedded_data != null) |
  .embedded_data
' | base64 -d > attachment.jpg

# For gzip-compressed attachments
cat chat_42.ndjson | jq -r '
  select(.content.components[]? | .type == "attachment" and .embedded_compression == "gzip") |
  .content.components[] |
  select(.type == "attachment" and .embedded_data != null) |
  .embedded_data
' | base64 -d | gunzip > attachment.jpg
```

#### Extract Zstd-Compressed Attachment

```bash
cat chat_42.ndjson | jq -r '
  select(.content.components[]? | .type == "attachment" and .embedded_compression == "zstd") |
  .content.components[] |
  select(.type == "attachment" and .embedded_data != null) |
  .embedded_data
' | base64 -d | zstd -d > attachment.pdf
```

#### Extract All Attachments from a Chat

```bash
#!/bin/bash
# extract_attachments.sh - Extract all embedded attachments from NDJSON file

input_file="$1"
output_dir="${2:-./extracted}"

mkdir -p "$output_dir"

jq -c 'select(.content.components[]? | .type == "attachment" and .embedded_data != null)' "$input_file" | \
while IFS= read -r message; do
  # Extract attachment components
  echo "$message" | jq -c '.content.components[] | select(.type == "attachment" and .embedded_data != null)' | \
  while IFS= read -r attachment; do
    # Get metadata
    filename=$(echo "$attachment" | jq -r '.filename // .transfer_name // "unknown"')
    compression=$(echo "$attachment" | jq -r '.embedded_compression // "none"')
    hash=$(echo "$attachment" | jq -r '.content_hash // ""')

    # Use hash for filename if available (avoids duplicates)
    if [ -n "$hash" ]; then
      ext="${filename##*.}"
      outfile="$output_dir/${hash:0:16}.${ext}"
    else
      outfile="$output_dir/$filename"
    fi

    # Skip if already extracted
    [ -f "$outfile" ] && continue

    # Extract and decompress based on method
    case "$compression" in
      "gzip")
        echo "$attachment" | jq -r '.embedded_data' | base64 -d | gunzip > "$outfile"
        ;;
      "zstd")
        echo "$attachment" | jq -r '.embedded_data' | base64 -d | zstd -d -o "$outfile"
        ;;
      "none")
        echo "$attachment" | jq -r '.embedded_data' | base64 -d > "$outfile"
        ;;
    esac

    echo "Extracted: $outfile"
  done
done
```

Usage:
```bash
chmod +x extract_attachments.sh
./extract_attachments.sh chat_42.ndjson ./my_attachments
```

#### List All Embedded Attachments with Metadata

```bash
cat chat_42.ndjson | jq -r '
  select(.content.components[]? | .type == "attachment" and .embedded_data != null) |
  {
    date: .metadata.date,
    sender: .sender.contact_name // .sender.identifier,
    attachments: [
      .content.components[] |
      select(.type == "attachment" and .embedded_data != null) |
      {
        filename: .filename // .transfer_name,
        size: .size_bytes,
        mime_type,
        compression: .embedded_compression,
        hash: .content_hash
      }
    ]
  }
'
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

### Using grep

Find messages containing a keyword:

```bash
grep -i "keyword" chat_42.ndjson | jq -c '.content.text'
```

## Architecture

Built as a standalone Rust binary using:

- **imessage-database** - Handles database queries and parsing (from imessage-exporter)
- **serde/serde_json** - JSON serialization
- **clap** - Command-line argument parsing
- **indicatif** - Progress indicators
- **chrono** - Date/time handling

The tool uses imessage-database as a library dependency, ensuring compatibility with the same iMessage database schemas (iOS 13 through iOS 18+).

## Comparison with imessage-exporter

| Feature | imessage-ndjson-exporter | imessage-exporter |
|---------|-------------------------|-------------------|
| Output Format | NDJSON (structured JSON) | HTML, TXT, PDF (planned) |
| Data Completeness | Lossless (all metadata) | Lossless (all metadata) |
| Processing | Easy (standard JSON tools) | Hard (need HTML parsing) |
| Human Readable | No (machine-first) | Yes (presentation-first) |
| Use Case | Data analysis, archival | Reading, sharing |
| Attachment Handling | Copy or embed (base64+compression) | Can copy files |

## Future Enhancements

Possible future additions (not currently implemented):

- **Attachment format conversion** - Convert HEIC to JPEG, MOV to MP4, etc. (currently stubbed)
- **NDJSON → HTML renderer** - Convert NDJSON back to HTML
- **NDJSON → TXT renderer** - Convert NDJSON to plain text
- **Incremental exports** - Only export new messages
- **File compression** - Automatic `.ndjson.gz` output for NDJSON files
- **Date filtering** - Export specific date ranges
- **Participant avatars** - Include contact photos in metadata

## License

GPL-3.0-or-later

## Credits

This tool builds on the excellent [imessage-exporter](https://github.com/ReagentX/imessage-exporter) project by Christopher Sardegna, which provides the database parsing library (imessage-database) and established patterns for handling iMessage data.

## Contributing

Issues and pull requests welcome! This is an independent tool separate from the main imessage-exporter project.
