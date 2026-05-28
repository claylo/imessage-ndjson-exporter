use anyhow::{Context, Result};
use imessage_db::imessage::entities::{Chat, Handle, Message};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::{BufWriter, Write as _},
    path::{Path, PathBuf},
};

use crate::{
    attachment_manager::{AttachmentManager, CompressionMode},
    avatar_manager::AvatarManager,
    contacts::ContactsIndex,
    db::Database,
    resolvers::{ContactResolver, TapbackResolver},
    serialization::{
        attachments::SerializableAttachment,
        chat::{SerializableChatContext, SerializableSender},
        content::{ContentComponent, SerializableContent, TextAttribute, TextEffect},
        message::{
            MessageMetadata, SerializableAnnouncement, SerializableGroupAction,
            SerializableMessage,
        },
        participant::SerializableParticipant,
        relationships::SerializableRelationships,
    },
};

pub struct NdjsonExporter {
    database_path: PathBuf,
    output_dir: PathBuf,
    custom_name: Option<String>,
    show_progress: bool,
    conversation_filter: Option<String>,
    contacts_path: Option<PathBuf>,
    copy_attachments: bool,
    convert_attachments: bool,
    attachments_dir: String,
    embed_attachments: bool,
    max_embed_size: usize,
    embed_compression: CompressionMode,
    include_avatars: bool,
    start_timestamp: Option<i64>, // Unix milliseconds (inclusive)
    end_timestamp: Option<i64>,   // Unix milliseconds (exclusive)
}

impl NdjsonExporter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        database_path: &Path,
        output_dir: &Path,
        custom_name: Option<String>,
        show_progress: bool,
        conversation_filter: Option<String>,
        contacts_path: Option<PathBuf>,
        copy_attachments: bool,
        convert_attachments: bool,
        attachments_dir: String,
        embed_attachments: bool,
        max_embed_size: usize,
        embed_compression: CompressionMode,
        include_avatars: bool,
        start_date: Option<String>,
        end_date: Option<String>,
    ) -> Result<Self> {
        use chrono::NaiveDate;

        // Parse start date to Unix milliseconds (inclusive)
        let start_timestamp = start_date
            .map(|s| parse_date_to_unix_ms(&s))
            .transpose()?;

        // Parse end date to Unix milliseconds (exclusive - add 1 day)
        let end_timestamp = end_date
            .map(|s| {
                let date = NaiveDate::parse_from_str(&s, "%Y-%m-%d")?;
                let next_day = date + chrono::Duration::days(1);
                parse_date_to_unix_ms(&next_day.format("%Y-%m-%d").to_string())
            })
            .transpose()?;

        Ok(Self {
            database_path: database_path.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            custom_name,
            show_progress,
            conversation_filter,
            contacts_path,
            copy_attachments,
            convert_attachments,
            attachments_dir,
            embed_attachments,
            max_embed_size,
            embed_compression,
            include_avatars,
            start_timestamp,
            end_timestamp,
        })
    }

    /// Resolve conversation filter to specific chat IDs
    fn resolve_filtered_chats(
        &self,
        db: &Database,
        contacts_index: &ContactsIndex,
        handles: &HashMap<i64, Handle>,
        chats: &HashMap<i64, Chat>,
    ) -> Result<BTreeSet<i64>> {
        let Some(ref filter) = self.conversation_filter else {
            return Ok(BTreeSet::new());
        };

        // Parse comma-separated filter terms
        let filter_terms: Vec<&str> = filter.split(',').map(|s| s.trim()).collect();

        // Build participants map (handle_id -> handle details)
        let participants: HashMap<i64, String> = handles
            .iter()
            .map(|(id, handle)| (*id, handle.id.clone()))
            .collect();

        // Build deduplication map (for ContactsIndex.build_participants_map)
        let deduped_handles: HashMap<i64, i64> = handles.keys().map(|&id| (id, id)).collect();

        // Use ContactsIndex to build participants map with Names
        let participants_with_names =
            contacts_index.build_participants_map(&participants, &deduped_handles);

        // Find matching handle_ids
        let mut included_handles: BTreeSet<i64> = BTreeSet::new();
        for name in participants_with_names.values() {
            for filter_term in &filter_terms {
                if name.contains(filter_term) {
                    included_handles.extend(&name.handle_ids);
                }
            }
        }

        // Build chatroom participants map from loaded chats
        let chatroom_participants = db.build_chatroom_participants(chats);

        // Find chats containing the selected handles
        let mut included_chatrooms: BTreeSet<i64> = BTreeSet::new();
        for (chat_id, participants) in &chatroom_participants {
            if !participants.is_disjoint(&included_handles) {
                included_chatrooms.insert(*chat_id);
            }
        }

        // For chats not yet included, check if they have a chat_identifier matching a handle
        for (chat_id, chat) in chats {
            if included_chatrooms.contains(chat_id) {
                continue;
            }

            // For 1:1 chats, the chat_identifier often IS the handle identifier
            if let Some(ref chat_ident) = chat.chat_identifier {
                for handle in handles.values() {
                    if included_handles.contains(&handle.rowid) && handle.id == *chat_ident {
                        included_chatrooms.insert(*chat_id);
                        break;
                    }
                }
            }
        }

        Ok(included_chatrooms)
    }

    pub fn export(&self) -> Result<()> {
        // Connect to database
        let db = Database::open(&self.database_path)?;

        // Validate converters if conversion is requested (strict mode)
        if self.convert_attachments {
            use crate::converters::{AudioConverter, Converter, ImageConverter, VideoConverter};

            let mut missing_tools = Vec::new();

            if ImageConverter::determine().is_none() {
                missing_tools.push("Image converter (install: brew install imagemagick)");
            }
            if VideoConverter::determine().is_none() {
                missing_tools.push("Video converter (install: brew install ffmpeg)");
            }
            if AudioConverter::determine().is_none() {
                missing_tools.push("Audio converter (ffmpeg or afconvert required)");
            }

            if !missing_tools.is_empty() {
                anyhow::bail!(
                    "Attachment conversion requested but required tools are not installed:\n\n  {}\n\n\
                     Install all required tools or remove --convert-attachments flag.",
                    missing_tools.join("\n  ")
                );
            }
        }

        // Build caches
        if self.show_progress {
            println!("Building caches...");
        }

        let (chats, handles, tapbacks, chatroom_participants) = self.build_caches(&db)?;

        // Build ContactsIndex if filter is specified
        let contacts_index = if self.conversation_filter.is_some() {
            match ContactsIndex::build(self.contacts_path.as_deref()) {
                Ok(index) => index,
                Err(e) => anyhow::bail!(
                    "Failed to load contacts database (required for -t filter): {}\n\
                     Use --contacts-path to specify custom location or remove -t flag.",
                    e
                ),
            }
        } else {
            ContactsIndex::empty()
        };

        // Apply conversation filter if specified
        let selected_chat_ids = if let Some(ref filter) = self.conversation_filter {
            let chat_ids = self.resolve_filtered_chats(&db, &contacts_index, &handles, &chats)?;

            if chat_ids.is_empty() {
                anyhow::bail!(
                    "Filter '{}' does not match any conversations",
                    filter
                );
            }

            if self.show_progress {
                println!("Filter matched {} conversations", chat_ids.len());
            }

            Some(chat_ids)
        } else {
            None
        };

        // Query avatar paths before moving contacts_index into ContactResolver
        let avatar_paths = if self.include_avatars {
            contacts_index
                .get_avatar_paths(self.contacts_path.as_deref(), None)
                .unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };

        // Build contact resolver (consumes contacts_index)
        let mut contact_resolver = ContactResolver::new(contacts_index, self.custom_name.clone());

        // Create attachment manager if copying or embedding is enabled
        let mut attachment_manager = if self.copy_attachments || self.embed_attachments {
            Some(AttachmentManager::new(
                &self.output_dir,
                self.attachments_dir.clone(),
                self.convert_attachments,
                self.database_path.clone(),
            ))
        } else {
            None
        };

        // Create avatar manager if avatars are enabled
        let mut avatar_manager = if self.include_avatars {
            Some(AvatarManager::new(&self.output_dir))
        } else {
            None
        };

        // Export messages by chat
        let total_chats = if let Some(ref selected) = selected_chat_ids {
            selected.len()
        } else {
            chats.len()
        };
        let progress = if self.show_progress {
            let pb = ProgressBar::new(total_chats as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} chats")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            Some(pb)
        } else {
            None
        };

        let mut total_messages = 0;
        for (chat_id, chat) in &chats {
            // Skip if not in selected set
            if let Some(ref selected) = selected_chat_ids
                && !selected.contains(chat_id) {
                    continue;
                }

            let message_count = self.export_chat(
                &db,
                *chat_id,
                chat,
                &handles,
                &tapbacks,
                &mut contact_resolver,
                &mut attachment_manager,
            )?;

            // Write participants file if avatars are enabled and messages were exported
            if self.include_avatars && message_count > 0 {
                self.write_participants_file(
                    *chat_id,
                    &handles,
                    &chatroom_participants,
                    &mut contact_resolver,
                    &mut avatar_manager,
                    &avatar_paths,
                )?;
            }

            total_messages += message_count;

            if let Some(ref pb) = progress {
                pb.inc(1);
            }
        }

        if let Some(pb) = progress {
            pb.finish_with_message("Export complete");
        }

        println!(
            "\n✅ Exported {} messages from {} chats",
            total_messages, total_chats
        );

        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn build_caches(
        &self,
        db: &Database,
    ) -> Result<(
        HashMap<i64, Chat>,
        HashMap<i64, Handle>,
        TapbackResolver,
        HashMap<i64, BTreeSet<i64>>,
    )> {
        // Build chat cache
        let chats = db.load_chats()?;

        // Build handle cache
        let handles = db.load_handles()?;

        // Build tapback cache
        let tapback_map = db.load_tapbacks()?;
        let mut tapbacks = TapbackResolver::new();
        for (guid, msgs) in tapback_map {
            for msg in msgs {
                tapbacks.add_tapback(guid.clone(), msg);
            }
        }

        // Build chatroom participants from loaded chats
        let chatroom_participants = db.build_chatroom_participants(&chats);

        Ok((chats, handles, tapbacks, chatroom_participants))
    }

    #[allow(clippy::too_many_arguments)]
    fn export_chat(
        &self,
        db: &Database,
        chat_id: i64,
        chat: &Chat,
        handles: &HashMap<i64, Handle>,
        tapbacks: &TapbackResolver,
        contact_resolver: &mut ContactResolver,
        attachment_manager: &mut Option<AttachmentManager>,
    ) -> Result<usize> {
        // Create output file for this chat
        let filename = format!("chat_{}.ndjson", chat_id);
        let output_path = self.output_dir.join(filename);
        let file = File::create(&output_path)
            .context(format!("Failed to create output file: {:?}", output_path))?;
        let mut writer = BufWriter::new(file);

        // Get messages for this chat using the chat's guid
        let messages = db.messages_for_chat(
            &chat.guid,
            self.start_timestamp,
            self.end_timestamp,
        )?;

        let mut message_count = 0;

        for msg in &messages {
            // Convert to serializable format
            let serializable = self.convert_message(
                msg,
                chat_id,
                chat,
                handles,
                tapbacks,
                contact_resolver,
                attachment_manager,
            )?;

            // Write as JSON
            serde_json::to_writer(&mut writer, &serializable)?;
            writeln!(&mut writer)?;

            message_count += 1;
        }

        // If no messages were exported, clean up the empty file
        if message_count == 0 {
            drop(writer);
            std::fs::remove_file(&output_path)?;
            return Ok(0);
        }

        writer.flush()?;

        Ok(message_count)
    }

    #[allow(clippy::too_many_arguments)]
    fn convert_message(
        &self,
        msg: &Message,
        chat_id: i64,
        chat: &Chat,
        handles: &HashMap<i64, Handle>,
        _tapbacks: &TapbackResolver,
        contact_resolver: &mut ContactResolver,
        attachment_manager: &mut Option<AttachmentManager>,
    ) -> Result<SerializableMessage> {
        // Determine message type
        let message_type = determine_message_type(msg);

        // Build metadata
        let metadata = MessageMetadata {
            rowid: msg.rowid,
            guid: msg.guid.clone(),
            date: format_timestamp_ms(msg.date),
            date_read: msg.date_read.map(|d| format_timestamp_ms(Some(d))),
            date_delivered: msg.date_delivered.map(|d| format_timestamp_ms(Some(d))),
            date_edited: msg.date_edited.map(|d| format_timestamp_ms(Some(d))),
            service: msg.service.as_deref().unwrap_or("Unknown").to_string(),
            is_from_me: msg.is_from_me,
            is_read: msg.is_read,
            chat_id: Some(chat_id),
            is_deleted: msg.date_retracted.is_some(),
        };

        // Build sender info
        let sender = self.build_sender(msg, handles, contact_resolver);

        // Build chat context
        let chat_context = self.build_chat_context(chat);

        // Build content
        let content = self.build_content(msg, chat_id, attachment_manager)?;

        // Build relationships
        let relationships = SerializableRelationships {
            thread_originator_guid: msg.thread_originator_guid.clone(),
            thread_originator_part: msg.thread_originator_part.clone(),
            num_replies: 0, // imessage-db doesn't track reply counts directly
            tapbacks: vec![],   // TODO: Implement tapback resolution
            edit_history: None, // TODO: Implement edit history
        };

        // Build announcement metadata
        let announcement = build_announcement(msg);

        Ok(SerializableMessage {
            message_type,
            metadata,
            sender,
            chat_context,
            content,
            relationships,
            expressive_effect: None, // TODO: Implement expressive effects
            announcement,
        })
    }

    fn build_sender(
        &self,
        msg: &Message,
        handles: &HashMap<i64, Handle>,
        contact_resolver: &mut ContactResolver,
    ) -> SerializableSender {
        if msg.is_from_me {
            let id = "Me".to_string();
            let name = contact_resolver.resolve_name(&id, true);
            return SerializableSender {
                handle_id: None,
                identifier: id,
                contact_name: name,
            };
        }

        let handle_id = msg.handle_id;
        if handle_id > 0
            && let Some(handle) = handles.get(&handle_id) {
                let id = handle.id.clone();
                let name = contact_resolver.resolve_name(&id, false);
                return SerializableSender {
                    handle_id: Some(handle_id),
                    identifier: id,
                    contact_name: name,
                };
            }

        SerializableSender {
            handle_id: if handle_id > 0 { Some(handle_id) } else { None },
            identifier: "Unknown".to_string(),
            contact_name: None,
        }
    }

    fn build_chat_context(&self, chat: &Chat) -> SerializableChatContext {
        let chat_identifier = chat
            .chat_identifier
            .clone()
            .unwrap_or_else(|| chat.guid.clone());

        let participants: Vec<String> = if chat.participants.is_empty() {
            vec![chat_identifier.clone()]
        } else {
            chat.participants.iter().map(|h| h.id.clone()).collect()
        };

        SerializableChatContext {
            chat_id: Some(chat.rowid),
            chat_identifier,
            display_name: chat.display_name.clone(),
            service_name: chat
                .service_name
                .as_deref()
                .unwrap_or("Unknown")
                .to_string(),
            participants,
        }
    }

    fn build_content(
        &self,
        msg: &Message,
        chat_id: i64,
        attachment_manager: &mut Option<AttachmentManager>,
    ) -> Result<SerializableContent> {
        let mut components = Vec::new();

        // Get message text - try msg.text first, then decode attributed_body
        let text = get_message_text(msg);

        // Build text component if there's text
        if let Some(ref text_str) = text
            && !text_str.is_empty() {
                let attributes = build_text_attributes(msg);
                components.push(ContentComponent::Text {
                    text: text_str.clone(),
                    attributes,
                });
            }

        // Handle retracted messages
        if msg.date_retracted.is_some() && text.is_none() && msg.attachments.is_empty() {
            components.push(ContentComponent::Retracted);
        }

        // Handle attachments (eager-loaded on the message)
        for att in &msg.attachments {
            let original_path = att
                .filename
                .as_ref()
                .map(|f| resolve_attachment_path(f));

            // Track converted MIME type
            let mut converted_mime_type: Option<String> = None;

            let (
                copied_path,
                copy_error,
                embedded_data,
                embedded_encoding,
                embedded_compression,
                content_hash,
            ) = if self.embed_attachments {
                // Embed mode
                if let Some(mgr) = attachment_manager.as_mut() {
                    match mgr.embed_attachment_from_path(
                        original_path.as_deref(),
                        att.mime_type.as_deref(),
                        self.embed_compression,
                        self.max_embed_size,
                    ) {
                        Ok(embedded) => (
                            None,
                            None,
                            Some(embedded.data),
                            Some(embedded.encoding),
                            Some(embedded.compression),
                            Some(embedded.content_hash),
                        ),
                        Err(err) => (None, Some(err), None, None, None, None),
                    }
                } else {
                    (None, None, None, None, None, None)
                }
            } else if self.copy_attachments {
                // Copy mode
                if let Some(mgr) = attachment_manager.as_mut() {
                    match mgr.copy_attachment_from_path(
                        original_path.as_deref(),
                        att.transfer_name.as_deref(),
                        att.filename.as_deref(),
                        att.mime_type.as_deref(),
                        att.is_sticker.unwrap_or(false),
                        chat_id,
                    ) {
                        Ok((path, new_mime)) => {
                            converted_mime_type = new_mime;
                            (Some(path), None, None, None, None, None)
                        }
                        Err(err) => (None, Some(err), None, None, None, None),
                    }
                } else {
                    (None, None, None, None, None, None)
                }
            } else {
                // Reference-in-place mode (default)
                (None, None, None, None, None, None)
            };

            let final_mime_type = converted_mime_type.or_else(|| att.mime_type.clone());

            let serializable = SerializableAttachment {
                guid: Some(att.guid.clone()),
                filename: att.filename.clone(),
                transfer_name: att.transfer_name.clone(),
                mime_type: final_mime_type,
                uti: att.uti.clone(),
                size_bytes: att.total_bytes,
                transcription: None, // Not available in imessage-db
                dimensions: None,    // Not available in imessage-db entity directly
                is_sticker: att.is_sticker.unwrap_or(false),
                sticker_metadata: None,
                original_path,
                copied_path,
                copy_error,
                embedded_data,
                embedded_encoding,
                embedded_compression,
                content_hash,
            };

            components.push(ContentComponent::Attachment(serializable));
        }

        // Handle app messages
        if let Some(ref bundle_id) = msg.balloon_bundle_id
            && !bundle_id.is_empty() {
                components.push(ContentComponent::App {
                    balloon_bundle_id: bundle_id.clone(),
                    app_name: None,
                    app_type: "balloon".to_string(),
                    metadata: None,
                });
            }

        Ok(SerializableContent {
            text,
            subject: msg.subject.clone(),
            components,
        })
    }

    /// Write participants file for a chat
    fn write_participants_file(
        &self,
        chat_id: i64,
        handles: &HashMap<i64, Handle>,
        chatroom_participants: &HashMap<i64, BTreeSet<i64>>,
        contact_resolver: &mut ContactResolver,
        avatar_manager: &mut Option<AvatarManager>,
        avatar_paths: &HashMap<String, PathBuf>,
    ) -> Result<()> {
        // Get participants for this chat from the chatroom_participants map
        let participant_ids = match chatroom_participants.get(&chat_id) {
            Some(ids) => ids,
            None => return Ok(()), // No participants for this chat
        };

        // Collect all participants for this chat
        let mut participants = Vec::new();

        for &handle_id in participant_ids {
            if let Some(handle) = handles.get(&handle_id) {
                let identifier = handle.id.clone();
                let contact_name = contact_resolver.resolve_name(&identifier, false);

                // Look up avatar path and copy if available
                let avatar_path = if let Some(source_path) = avatar_paths.get(&identifier) {
                    if let Some(mgr) = avatar_manager.as_mut() {
                        mgr.copy_avatar(source_path)
                    } else {
                        None
                    }
                } else {
                    None
                };

                participants.push(SerializableParticipant {
                    handle_id,
                    identifier,
                    contact_name,
                    avatar_path,
                });
            }
        }

        // Write participants to NDJSON file
        let filename = if let Some(ref custom_name) = self.custom_name {
            format!("chat_{}_{}_participants.ndjson", chat_id, custom_name)
        } else {
            format!("chat_{}_participants.ndjson", chat_id)
        };
        let filepath = self.output_dir.join(&filename);
        let file = File::create(&filepath).with_context(|| {
            format!("Failed to create participants file: {}", filepath.display())
        })?;
        let mut writer = BufWriter::new(file);

        for participant in participants {
            let json =
                serde_json::to_string(&participant).context("Failed to serialize participant")?;
            writeln!(writer, "{}", json).context("Failed to write participant to file")?;
        }

        writer
            .flush()
            .context("Failed to flush participants file")?;

        Ok(())
    }
}

/// Determine message type from message fields.
fn determine_message_type(msg: &Message) -> String {
    // Tapback: has associated_message_type set
    if msg.associated_message_type.is_some() {
        return "tapback".to_string();
    }

    // Edited: has date_edited set
    if msg.date_edited.is_some() {
        return "edited".to_string();
    }

    // Announcement: group actions (item_type != 0) or group_title changes
    if msg.item_type != 0 || msg.group_title.is_some() {
        return "announcement".to_string();
    }

    "normal".to_string()
}

/// Build announcement from message fields.
fn build_announcement(msg: &Message) -> Option<SerializableAnnouncement> {
    // Fully unsent (retracted with no content)
    if msg.date_retracted.is_some() && msg.text.is_none() {
        return Some(SerializableAnnouncement::FullyUnsent);
    }

    // Group actions based on item_type and group_action_type
    // item_type values from iMessage:
    // 0 = normal message
    // 1 = participant change
    // 2 = group name/photo change
    // 3 = group action
    match msg.item_type {
        1 => {
            // Participant changes
            match msg.group_action_type {
                0 => Some(SerializableAnnouncement::GroupAction(
                    SerializableGroupAction::ParticipantAdded {
                        participant_handle_id: msg.other_handle,
                    },
                )),
                1 => Some(SerializableAnnouncement::GroupAction(
                    SerializableGroupAction::ParticipantRemoved {
                        participant_handle_id: msg.other_handle,
                    },
                )),
                _ => None,
            }
        }
        2 => {
            // Group name change
            if let Some(ref name) = msg.group_title {
                Some(SerializableAnnouncement::GroupAction(
                    SerializableGroupAction::NameChange {
                        new_name: name.clone(),
                    },
                ))
            } else {
                // Group icon changed
                Some(SerializableAnnouncement::GroupAction(
                    SerializableGroupAction::GroupIconChanged,
                ))
            }
        }
        3 => {
            // Audio message kept
            Some(SerializableAnnouncement::AudioMessageKept)
        }
        _ => None,
    }
}

/// Get message text, trying msg.text first, then decoding attributed_body.
fn get_message_text(msg: &Message) -> Option<String> {
    // msg.text is already populated by imessage-db
    if let Some(ref text) = msg.text
        && !text.is_empty() {
            return Some(text.clone());
        }

    // Fallback: decode attributed_body blob
    if let Some(ref body) = msg.attributed_body
        && !body.is_empty()
            && let Some(text) = imessage_core::typedstream::extract_text(body)
                && !text.is_empty() {
                    return Some(text);
                }

    None
}

/// Build text attributes from attributed_body blob.
fn build_text_attributes(msg: &Message) -> Vec<TextAttribute> {
    let Some(ref body) = msg.attributed_body else {
        return vec![];
    };

    if body.is_empty() {
        return vec![];
    }

    let Some(decoded) = imessage_core::typedstream::decode_attributed_body(body) else {
        return vec![];
    };

    // decoded is a JSON array of NSAttributedString objects
    // Each has "string" and "runs" fields
    let mut attributes = Vec::new();

    if let Some(items) = decoded.as_array() {
        for item in items {
            if let Some(runs) = item.get("runs").and_then(|r| r.as_array()) {
                for run in runs {
                    let range = run.get("range").and_then(|r| r.as_array());
                    let attrs = run.get("attributes").and_then(|a| a.as_object());

                    if let (Some(range), Some(attrs)) = (range, attrs) {
                        let start = range.first().and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        let length = range.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        let end = start + length;

                        let mut effects = Vec::new();

                        // Map known attribute keys to TextEffect variants
                        for (key, value) in attrs {
                            match key.as_str() {
                                "__kIMMessagePartAttributeName" => {
                                    // Mention
                                    if let Some(id) = value.as_str() {
                                        effects.push(TextEffect::Mention {
                                            identifier: id.to_string(),
                                        });
                                    }
                                }
                                "__kIMLinkAttributeName" | "NSLink" => {
                                    if let Some(url) = value.as_str() {
                                        effects.push(TextEffect::Link {
                                            url: url.to_string(),
                                        });
                                    }
                                }
                                "__kIMOneTimeCodeAttributeName" => {
                                    effects.push(TextEffect::OTP);
                                }
                                "__kIMDataDetectorResultAttributeName" => {
                                    effects.push(TextEffect::Conversion);
                                }
                                _ => {
                                    // Other attributes map to Default
                                }
                            }
                        }

                        if !effects.is_empty() {
                            attributes.push(TextAttribute {
                                start,
                                end,
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

/// Resolve attachment path, expanding `~/` to home directory.
fn resolve_attachment_path(filename: &str) -> String {
    if filename.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        filename.replacen("~/", &format!("{}/", home), 1)
    } else {
        filename.to_string()
    }
}

/// Format a Unix millisecond timestamp to ISO 8601 string.
fn format_timestamp_ms(timestamp_ms: Option<i64>) -> String {
    let Some(ms) = timestamp_ms else {
        return String::new();
    };

    if ms == 0 {
        return String::new();
    }

    let secs = ms / 1000;
    let nsecs = ((ms % 1000) * 1_000_000) as u32;

    let datetime = chrono::DateTime::from_timestamp(secs, nsecs)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());

    datetime.format("%Y-%m-%dT%H:%M:%S%z").to_string()
}

/// Parse a date string (YYYY-MM-DD) to Unix milliseconds.
fn parse_date_to_unix_ms(date_str: &str) -> anyhow::Result<i64> {
    use chrono::NaiveDate;

    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").context(format!(
        "Invalid date format '{}'. Expected YYYY-MM-DD",
        date_str
    ))?;

    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Failed to create datetime from date"))?;

    // Convert to Unix milliseconds
    let unix_ms = datetime.and_utc().timestamp_millis();

    Ok(unix_ms)
}
