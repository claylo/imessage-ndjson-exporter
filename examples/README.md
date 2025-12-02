# Export Examples

This directory contains example NDJSON exports demonstrating the full range of message types and features supported by the iMessage NDJSON exporter.

## File Organization

Each example file represents a chat export (`chat_{id}.ndjson`) with one message per line. All examples use realistic data structures based on actual iMessage database schemas.

## Example Files

### Basic Message Types

- **`basic_text.ndjson`** - Simple text messages
- **`rich_text.ndjson`** - Messages with text effects (bold, italic, underline, strikethrough)
- **`links_and_mentions.ndjson`** - Messages with URLs, @mentions, and OTP codes
- **`subject_lines.ndjson`** - Messages with subject lines (iMessage feature)

### Attachments

- **`photo_attachments.ndjson`** - Photo messages (HEIC, JPEG, PNG)
- **`video_attachments.ndjson`** - Video messages (MOV, MP4)
- **`audio_attachments.ndjson`** - Audio messages and voice memos (CAF, M4A)
- **`stickers.ndjson`** - Sticker messages (static and animated)
- **`multiple_attachments.ndjson`** - Messages with multiple attachments

### Attachment Modes

- **`attachments_reference.ndjson`** - Attachments in reference mode (original paths)
- **`attachments_copied.ndjson`** - Attachments in copy mode (relative paths with hashing)
- **`attachments_embedded.ndjson`** - Attachments in embed mode (base64-encoded data)

### Interactions and Relationships

- **`reactions.ndjson`** - Messages with tapback reactions (❤️, 👍, 👎, 😂, ‼️, ❓)
- **`thread_replies.ndjson`** - Threaded conversations with replies
- **`edit_history.ndjson`** - Edited messages with edit metadata
- **`deleted_messages.ndjson`** - Deleted/unsent messages

### Special Message Types

- **`group_announcements.ndjson`** - Group chat system messages (name changes, participants added/removed)
- **`expressive_effects.ndjson`** - Messages with bubble/screen effects (slam, loud, gentle, etc.)
- **`app_messages.ndjson`** - App integration messages (shared locations, Apple Pay, etc.)
- **`retracted_content.ndjson`** - Messages with retracted/removed content

### Service Types

- **`service_types.ndjson`** - Messages across different services (iMessage, SMS, RCS, Satellite)

### Complete Conversations

- **`group_chat_complete.ndjson`** - Full group conversation with mixed message types
- **`one_on_one_complete.ndjson`** - Complete 1:1 conversation with various features

## Understanding the NDJSON Format

Each line in an `.ndjson` file is a complete, valid JSON object representing one message. The format is:

```json
{"guid":"...","timestamp":"...","sender":"...","content":{...},"metadata":{...},"relationships":{...}}
{"guid":"...","timestamp":"...","sender":"...","content":{...},"metadata":{...},"relationships":{...}}
```

## Message Structure

All exported messages follow this structure:

```json
{
  "guid": "unique-message-identifier",
  "timestamp": "2025-12-01T10:30:00-08:00",
  "sender": {
    "handle_id": 42,
    "identifier": "+12345678901",
    "contact_name": "John Doe"
  },
  "content": {
    "subject": null,
    "components": [...]
  },
  "metadata": {
    "service": "iMessage",
    "is_from_me": false,
    "is_read": true,
    "is_delivered": true,
    "is_sent": true,
    "is_emote": false,
    "is_deleted": false,
    "message_type": "standard",
    "error_code": null,
    "expressive_effect": null
  },
  "relationships": {
    "thread_originator_guid": null,
    "reply_to_guid": null,
    "tapbacks": [],
    "edits": []
  }
}
```

## Key Features Demonstrated

### Content Components

Messages can contain multiple components:
- **Text** - Plain or attributed text with styling/effects
- **Attachment** - Photos, videos, audio, files
- **App** - App integrations (Maps, Apple Pay, etc.)
- **Retracted** - Placeholder for removed content

### Text Attributes

Text can have ranges with effects:
- **Links** - URLs with ranges
- **Mentions** - @-mentions of participants
- **OTP** - One-time password detection
- **Styles** - Bold, italic, underline, strikethrough

### Tapback Reactions

Six standard reactions plus custom emoji:
- `loved` (❤️)
- `liked` (👍)
- `disliked` (👎)
- `laughed` (😂)
- `emphasized` (‼️)
- `questioned` (❓)
- Custom emoji reactions

### Service Types

- `iMessage` - Apple's encrypted messaging
- `SMS` - Standard text messages
- `RCS` - Rich Communication Services
- `Satellite` - Emergency SOS satellite messages

### Message Types

- `standard` - Normal messages
- `announcement` - System/group announcements
- `edited` - Messages that have been edited

### Group Announcements

- `unknown` - Unknown action
- `name_change` - Group name changed
- `photo_change` - Group photo changed
- `participant_added` - Someone joined
- `participant_removed` - Someone left/was removed
- `audio_kept` - Audio message kept after expiration

## Usage

These examples can be used for:
1. **Understanding export format** - See what your exported data will look like
2. **Testing parsers** - Validate tools that consume NDJSON exports
3. **Documentation** - Reference for building integrations
4. **Regression testing** - Verify export functionality remains consistent

## Reading Examples

To view examples in a readable format:

```bash
# Pretty-print a single message
head -1 examples/basic_text.ndjson | jq .

# View all messages in a file
jq . examples/basic_text.ndjson

# Count messages by type
jq -r '.metadata.message_type' examples/group_chat_complete.ndjson | sort | uniq -c

# Extract all text content
jq -r '.content.components[] | select(.text) | .text' examples/basic_text.ndjson
```
