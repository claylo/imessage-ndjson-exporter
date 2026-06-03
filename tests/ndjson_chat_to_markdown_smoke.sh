#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT_DIR/scripts/ndjson-chat-to-markdown.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

CHAT="$TMP_DIR/chat_7.ndjson"
PARTICIPANTS="$TMP_DIR/chat_7_participants.ndjson"
EXPECTED="$TMP_DIR/expected.md"
ACTUAL="$TMP_DIR/actual.md"

cat >"$CHAT" <<'JSON'
{"message_type":"normal","metadata":{"rowid":1,"guid":"msg-1","date":"2026-01-02T03:04:05Z","service":"iMessage","is_from_me":false,"is_read":true,"chat_id":7,"is_deleted":false},"sender":{"handle_id":10,"identifier":"+15550000001"},"chat_context":{"chat_id":7,"chat_identifier":"chat-7","display_name":"Family Trip","service_name":"iMessage","participants":["+15550000001","me@example.com"]},"content":{"text":"Hello from Alice.","components":[{"type":"text","text":"Hello from Alice.","attributes":[]},{"type":"attachment","filename":"photo.jpg","mime_type":"image/jpeg","size_bytes":12345,"copied_path":"attachments/photo.jpg"}]},"relationships":{"num_replies":1,"tapbacks":[{"tapback_type":"loved","added_by":{"handle_id":1,"identifier":"me@example.com"},"timestamp":"2026-01-02T03:05:00Z","message_part_index":0,"is_from_me":true}]}}
{"message_type":"edited","metadata":{"rowid":2,"guid":"msg-2","date":"2026-01-02T03:06:00Z","date_edited":"2026-01-02T03:07:00Z","service":"iMessage","is_from_me":true,"is_read":true,"chat_id":7,"is_deleted":false},"sender":{"handle_id":1,"identifier":"me@example.com"},"chat_context":{"chat_id":7,"chat_identifier":"chat-7","display_name":"Family Trip","service_name":"iMessage","participants":["+15550000001","me@example.com"]},"content":{"components":[{"type":"text","text":"Reply from me.","attributes":[]}]},"relationships":{"thread_originator_guid":"msg-1","thread_originator_part":"0","num_replies":0,"tapbacks":[],"edit_history":{"status":"edited","versions":[{"text":"Reply frm me.","timestamp":"2026-01-02T03:06:30Z","components":[]}]}}}
JSON

cat >"$PARTICIPANTS" <<'JSON'
{"handle_id":10,"identifier":"+15550000001","contact_name":"Alice Example","avatar_path":"avatars/alice.jpg"}
{"handle_id":1,"identifier":"me@example.com","contact_name":"Me"}
JSON

cat >"$EXPECTED" <<'MARKDOWN'
# Family Trip

- Source: `chat_7.ndjson`
- Chat ID: `7`
- Service: iMessage

## Participants

- Alice Example (`+15550000001`) - avatar: `avatars/alice.jpg`
- Me (`me@example.com`)

## Messages

### 2026-01-02T03:04:05Z - Alice Example

Hello from Alice.

Attachments:
- photo.jpg (image/jpeg, 12345 bytes) - `attachments/photo.jpg`

Tapbacks:
- loved by Me at 2026-01-02T03:05:00Z

Replies: 1

### 2026-01-02T03:06:00Z - Me

Reply from me.

Reply to: `msg-1` part `0`

Edit history: edited
- 2026-01-02T03:06:30Z: Reply frm me.
MARKDOWN

bash "$SCRIPT" --participants "$PARTICIPANTS" "$CHAT" >"$ACTUAL"
diff -u "$EXPECTED" "$ACTUAL"
