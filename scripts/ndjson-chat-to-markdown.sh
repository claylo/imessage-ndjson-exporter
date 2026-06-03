#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: ndjson-chat-to-markdown.sh [--participants FILE] CHAT_NDJSON [OUTPUT_MD]

Convert an iMessage NDJSON chat export to a Markdown document.

Arguments:
  CHAT_NDJSON          Message export file to read. Use - for stdin.
  OUTPUT_MD           Optional Markdown output path. Defaults to stdout.

Options:
  --participants FILE  Optional chat_XX_participants.ndjson sidecar.
  -h, --help           Show this help text.
USAGE
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

need_arg() {
  local flag="$1"
  local value="${2:-}"
  [[ -n "$value" ]] || die "$flag requires a value"
}

participants_file=""
chat_file=""
output_file=""

while (($#)); do
  case "$1" in
    --participants)
      need_arg "$1" "${2:-}"
      participants_file="$2"
      shift 2
      ;;
    --participants=*)
      participants_file="${1#--participants=}"
      [[ -n "$participants_file" ]] || die "--participants requires a value"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      die "unknown option: $1"
      ;;
    *)
      if [[ -z "$chat_file" ]]; then
        chat_file="$1"
      elif [[ -z "$output_file" ]]; then
        output_file="$1"
      else
        die "unexpected argument: $1"
      fi
      shift
      ;;
  esac
done

while (($#)); do
  if [[ -z "$chat_file" ]]; then
    chat_file="$1"
  elif [[ -z "$output_file" ]]; then
    output_file="$1"
  else
    die "unexpected argument: $1"
  fi
  shift
done

[[ -n "$chat_file" ]] || {
  usage >&2
  exit 2
}

command -v jq >/dev/null 2>&1 || die "jq is required"

if [[ "$chat_file" != "-" ]]; then
  [[ -r "$chat_file" ]] || die "cannot read chat file: $chat_file"
  source_name="${chat_file##*/}"
else
  source_name="stdin"
fi

if [[ -n "$participants_file" ]]; then
  [[ -r "$participants_file" ]] || die "cannot read participants file: $participants_file"
fi

read -r -d '' JQ_PROGRAM <<'JQ' || true
def first_nonempty:
  map(select(. != null and (tostring | length) > 0))[0] // null;

def participant_label:
  if ((.contact_name? // "") != "") then .contact_name
  elif ((.identifier? // "") != "") then .identifier
  elif (.handle_id? != null) then "handle " + (.handle_id | tostring)
  else "Unknown participant"
  end;

def lookup_by_handle($id):
  ([
    $participants[]?
    | select(.handle_id? != null and ((.handle_id | tostring) == ($id | tostring)))
    | participant_label
  ][0] // null);

def lookup_by_identifier($identifier):
  ([
    $participants[]?
    | select((.identifier? // "") == ($identifier | tostring))
    | participant_label
  ][0] // null);

def sender_label($sender):
  if (($sender.contact_name? // "") != "") then $sender.contact_name
  elif ($sender.handle_id? != null) and (lookup_by_handle($sender.handle_id) != null) then lookup_by_handle($sender.handle_id)
  elif (($sender.identifier? // "") != "") and (lookup_by_identifier($sender.identifier) != null) then lookup_by_identifier($sender.identifier)
  elif (($sender.identifier? // "") != "") then $sender.identifier
  elif ($sender.handle_id? != null) then "handle " + ($sender.handle_id | tostring)
  else "Unknown sender"
  end;

def message_time:
  ([.metadata.date?, .timestamp?, "Unknown time"] | first_nonempty | tostring);

def message_text:
  if ((.content.text? // "") != "") then .content.text
  else
    [
      .content.components[]?
      | select(((.type? // "") == "text") or ((.text? != null) and (.filename? == null)))
      | .text?
      | select(. != null and (tostring | length) > 0)
    ]
    | join("\n\n")
  end;

def attachment_lines:
  [
    .content.components[]?
    | select(((.type? // "") == "attachment") or ((.filename? != null) and (.text? == null)))
    | ([.filename?, .transfer_name?, .guid?, "attachment"] | first_nonempty | tostring) as $name
    | ([.mime_type?, (if .size_bytes? != null then ((.size_bytes | tostring) + " bytes") else null end)]
        | map(select(. != null and (tostring | length) > 0))
        | if length > 0 then " (" + join(", ") + ")" else "" end) as $meta
    | ([.copied_path?, .original_path?, .embedded_encoding?] | first_nonempty) as $path
    | "- " + $name + $meta + (if $path != null then " - `" + ($path | tostring) + "`" else "" end)
  ];

def tapback_lines:
  [
    .relationships.tapbacks[]?
    | ([.tapback_type?, .reaction_type?, "tapback"] | first_nonempty | tostring) as $type
    | (if .emoji? != null then " " + (.emoji | tostring) else "" end) as $emoji
    | (if .added_by? != null then sender_label(.added_by)
       else sender_label({
         handle_id: .from_handle_id?,
         identifier: .from_identifier?,
         contact_name: .from_contact_name?
       })
       end) as $by
    | ([.timestamp?, .added_at?] | first_nonempty) as $time
    | "- " + $type + $emoji + " by " + $by + (if $time != null then " at " + ($time | tostring) else "" end)
  ];

def reply_block:
  ([.relationships.thread_originator_guid?, .relationships.reply_to_guid?] | first_nonempty) as $guid
  | if $guid == null then null
    else
      "Reply to: `" + ($guid | tostring) + "`" +
      (if .relationships.thread_originator_part? != null then " part `" + (.relationships.thread_originator_part | tostring) + "`" else "" end)
    end;

def replies_block:
  if ((.relationships.num_replies? // 0) > 0) then "Replies: " + (.relationships.num_replies | tostring)
  else null
  end;

def edit_history_block:
  if .relationships.edit_history? != null then
    (["Edit history: " + ((.relationships.edit_history.status? // "available") | tostring)]
      + [
        .relationships.edit_history.versions[]?
        | "- " + ((.timestamp? // "unknown time") | tostring) + ": " + ((.text? // "") | tostring)
      ])
    | join("\n")
  elif ((.relationships.edits? // []) | length) > 0 then
    (["Edits:"]
      + [
        .relationships.edits[]?
        | "- " + ((.edited_at? // "unknown time") | tostring) + ": " + ((.previous_text? // "") | tostring)
      ])
    | join("\n")
  else null
  end;

def attachment_block:
  attachment_lines as $lines
  | if ($lines | length) > 0 then "Attachments:\n" + ($lines | join("\n")) else null end;

def tapback_block:
  tapback_lines as $lines
  | if ($lines | length) > 0 then "Tapbacks:\n" + ($lines | join("\n")) else null end;

def message_markdown:
  [
    "### " + message_time + " - " + sender_label(.sender // {}),
    (message_text | select(length > 0)),
    attachment_block,
    tapback_block,
    reply_block,
    replies_block,
    edit_history_block
  ]
  | map(select(. != null and (tostring | length) > 0))
  | join("\n\n");

def participant_lines($first):
  if ($participants | length) > 0 then
    [
      $participants[]?
      | "- " + participant_label + " (`" + ((.identifier? // ("handle " + (.handle_id | tostring))) | tostring) + "`)" +
        (if ((.avatar_path? // "") != "") then " - avatar: `" + (.avatar_path | tostring) + "`" else "" end)
    ]
  else
    [
      $first.chat_context.participants[]?
      | "- `" + (tostring) + "`"
    ]
  end;

. as $messages
| ($messages[0] // {}) as $first
| ([
    $first.chat_context.display_name?,
    $first.chat_context.chat_identifier?,
    (if $first.metadata.chat_id? != null then "Chat " + ($first.metadata.chat_id | tostring) else null end),
    $source_name
  ] | first_nonempty | tostring) as $title
| ([
    "- Source: `" + $source_name + "`",
    (if $first.metadata.chat_id? != null then "- Chat ID: `" + ($first.metadata.chat_id | tostring) + "`" else null end),
    (if ([$first.chat_context.service_name?, $first.metadata.service?] | first_nonempty) != null
      then "- Service: " + (([$first.chat_context.service_name?, $first.metadata.service?] | first_nonempty) | tostring)
      else null
     end)
  ] | map(select(. != null)) | join("\n")) as $metadata
| (participant_lines($first)) as $participants_section
| [
    "# " + $title,
    $metadata,
    (if ($participants_section | length) > 0 then "## Participants\n\n" + ($participants_section | join("\n")) else null end),
    "## Messages\n\n" + ([$messages[]? | message_markdown] | join("\n\n"))
  ]
| map(select(. != null and (tostring | length) > 0))
| join("\n\n")
JQ

run_jq() {
  if [[ -n "$participants_file" ]]; then
    jq -r -s --arg source_name "$source_name" --slurpfile participants "$participants_file" "$JQ_PROGRAM" "$chat_file"
  else
    jq -r -s --arg source_name "$source_name" --argjson participants '[]' "$JQ_PROGRAM" "$chat_file"
  fi
}

if [[ -n "$output_file" ]]; then
  run_jq >"$output_file"
else
  run_jq
fi
