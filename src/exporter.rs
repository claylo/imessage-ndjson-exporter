use anyhow::{Context, Result};
use imessage_database::{
    message_types::variants::Announcement,
    tables::{
        attachment::Attachment,
        chat::Chat,
        chat_handle::ChatToHandle,
        handle::Handle,
        messages::{models::GroupAction, Message},
        table::{get_connection, Cacheable, Table},
    },
    util::{platform::Platform, query_context::QueryContext},
};
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::Connection;
use serde_json;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::File,
    io::{BufWriter, Write as _},
    path::{Path, PathBuf},
};

use crate::{
    attachment_manager::{AttachmentManager, CompressionMode},
    avatar_manager::AvatarManager,
    contacts::ContactsIndex,
    resolvers::{ContactResolver, TapbackResolver},
    serialization::{
        attachments::{AttachmentDimensions, SerializableAttachment},
        chat::{SerializableChatContext, SerializableSender},
        content::{ContentComponent, SerializableContent, TextAttribute, TextEffect},
        message::{
            MessageMetadata, SerializableAnnouncement, SerializableGroupAction, SerializableMessage,
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
}

impl NdjsonExporter {
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
    ) -> Result<Self> {
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
        })
    }

    /// Resolve conversation filter to specific chat IDs and handle IDs
    ///
    /// Returns (selected_chat_ids, selected_handle_ids)
    fn resolve_filtered_chats(
        &self,
        db: &Connection,
        contacts_index: &ContactsIndex,
        handles: &HashMap<i32, Handle>,
        chats: &HashMap<i32, Chat>,
    ) -> Result<BTreeSet<i32>> {
        let Some(ref filter) = self.conversation_filter else {
            return Ok(BTreeSet::new());
        };

        // Parse comma-separated filter terms
        let filter_terms: Vec<&str> = filter.split(',').map(|s| s.trim()).collect();

        // Build participants map (handle_id -> handle details)
        let participants: HashMap<i32, String> = handles
            .iter()
            .map(|(id, handle)| (*id, handle.id.clone()))
            .collect();

        // Build deduplication map (for ContactsIndex.build_participants_map)
        // For simplicity, map each handle to itself (no deduplication)
        let deduped_handles: HashMap<i32, i32> = handles.keys().map(|&id| (id, id)).collect();

        // Use ContactsIndex to build participants map with Names
        let participants_with_names =
            contacts_index.build_participants_map(&participants, &deduped_handles);

        // Find matching handle_ids
        let mut included_handles: HashSet<i32> = HashSet::new();
        for (_deduped_id, name) in &participants_with_names {
            for filter_term in &filter_terms {
                if name.contains(filter_term) {
                    included_handles.extend(&name.handle_ids);
                }
            }
        }

        // Build chatroom participants map (chat_id -> set of handle_ids in that chat)
        let mut chatroom_participants: HashMap<i32, HashSet<i32>> = HashMap::new();

        // Query chat_handle_join table to get participants for each chat
        let mut stmt =
            db.prepare("SELECT chat_id, handle_id FROM chat_handle_join ORDER BY chat_id")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let chat_id: i32 = row.get(0)?;
            let handle_id: i32 = row.get(1)?;
            chatroom_participants
                .entry(chat_id)
                .or_insert_with(HashSet::new)
                .insert(handle_id);
        }

        // Find chats containing the selected handles
        let mut included_chatrooms: BTreeSet<i32> = BTreeSet::new();
        for (chat_id, participants) in &chatroom_participants {
            // Include chat if it contains any of the selected handles
            if !participants.is_disjoint(&included_handles) {
                included_chatrooms.insert(*chat_id);
            }
        }

        // Also check all chats to see if they have messages from selected handles
        // (handles 1:1 chats that may not be in chat_handle_join)
        for chat_id in chats.keys() {
            if included_chatrooms.contains(chat_id) {
                continue; // Already included
            }

            // Check if this chat has any messages from our selected handles
            // Use chat_message_join to link messages to chats
            for &handle_id in &included_handles {
                let mut stmt = db.prepare(
                    "SELECT 1 FROM message m
                     INNER JOIN chat_message_join cmj ON m.ROWID = cmj.message_id
                     WHERE cmj.chat_id = ? AND m.handle_id = ? LIMIT 1",
                )?;
                let has_message: Result<i32, _> =
                    stmt.query_row([chat_id, &handle_id], |row| row.get(0));

                if has_message.is_ok() {
                    included_chatrooms.insert(*chat_id);
                    break;
                }
            }
        }

        Ok(included_chatrooms)
    }

    pub fn export(&self) -> Result<()> {
        // Connect to database
        let db = get_connection(&self.database_path)
            .map_err(|e| anyhow::anyhow!("Failed to connect to iMessage database: {:?}", e))?;

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

        // Get timezone offset (not critical if it fails, use 0)
        let offset = 0i64;

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
        let selected_chat_ids = if self.conversation_filter.is_some() {
            let chat_ids = self.resolve_filtered_chats(&db, &contacts_index, &handles, &chats)?;

            if chat_ids.is_empty() {
                anyhow::bail!(
                    "Filter '{}' does not match any conversations",
                    self.conversation_filter.as_ref().unwrap()
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
                Platform::macOS,
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
            if let Some(ref selected) = selected_chat_ids {
                if !selected.contains(chat_id) {
                    continue;
                }
            }

            let message_count = self.export_chat(
                &db,
                *chat_id,
                chat,
                &handles,
                &tapbacks,
                &mut contact_resolver,
                &mut attachment_manager,
                offset,
            )?;

            // Write participants file if avatars are enabled
            if self.include_avatars {
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

    fn build_caches(
        &self,
        db: &Connection,
    ) -> Result<(
        HashMap<i32, Chat>,
        HashMap<i32, Handle>,
        TapbackResolver,
        HashMap<i32, BTreeSet<i32>>,
    )> {
        // Build chat cache
        let mut chats = HashMap::new();
        let _ = Chat::stream(db, |chat_result| {
            if let Ok(chat) = chat_result {
                chats.insert(chat.rowid, chat);
            }
            Ok::<(), anyhow::Error>(())
        });

        // Build handle cache
        let mut handles = HashMap::new();
        let _ = Handle::stream(db, |handle_result| {
            if let Ok(handle) = handle_result {
                handles.insert(handle.rowid, handle);
            }
            Ok::<(), anyhow::Error>(())
        });

        // Build tapback cache
        let mut tapbacks = TapbackResolver::new();
        let _ = Message::stream(db, |msg_result| {
            if let Ok(msg) = msg_result {
                if msg.is_tapback() {
                    if let Some(ref associated_guid) = msg.associated_message_guid {
                        tapbacks.add_tapback(associated_guid.clone(), msg);
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        });

        // Build chat-to-handle cache (participant membership)
        let chatroom_participants = ChatToHandle::cache(db).map_err(|e| {
            anyhow::anyhow!("Failed to cache chat-to-handle relationships: {:?}", e)
        })?;

        Ok((chats, handles, tapbacks, chatroom_participants))
    }

    fn export_chat(
        &self,
        db: &Connection,
        chat_id: i32,
        chat: &Chat,
        handles: &HashMap<i32, Handle>,
        tapbacks: &TapbackResolver,
        contact_resolver: &mut ContactResolver,
        attachment_manager: &mut Option<AttachmentManager>,
        offset: i64,
    ) -> Result<usize> {
        // Create output file for this chat
        let filename = format!("chat_{}.ndjson", chat_id);
        let output_path = self.output_dir.join(filename);
        let file = File::create(&output_path)
            .context(format!("Failed to create output file: {:?}", output_path))?;
        let mut writer = BufWriter::new(file);

        // Get messages for this chat
        let mut query_context = QueryContext::default();
        query_context.selected_chat_ids = Some(BTreeSet::from([chat_id]));

        let mut message_count = 0;
        Message::stream(db, |msg_result| {
            if let Ok(mut msg) = msg_result {
                // Only export messages in this chat
                if msg.chat_id != Some(chat_id) {
                    return Ok::<(), anyhow::Error>(());
                }

                // Generate message text and components
                if let Err(e) = msg.generate_text(db) {
                    eprintln!(
                        "Warning: Failed to generate text for message {}: {}",
                        msg.guid, e
                    );
                }

                // Convert to serializable format
                let serializable = self.convert_message(
                    db,
                    &msg,
                    chat_id,
                    chat,
                    handles,
                    tapbacks,
                    contact_resolver,
                    attachment_manager,
                    offset,
                )?;

                // Write as JSON
                serde_json::to_writer(&mut writer, &serializable)?;
                writeln!(&mut writer)?;

                message_count += 1;
            }
            Ok::<(), anyhow::Error>(())
        })
        .map_err(|e| anyhow::anyhow!("Failed to stream messages: {:?}", e))?;

        writer.flush()?;

        Ok(message_count)
    }

    fn convert_message(
        &self,
        db: &Connection,
        msg: &Message,
        chat_id: i32,
        chat: &Chat,
        handles: &HashMap<i32, Handle>,
        _tapbacks: &TapbackResolver,
        contact_resolver: &mut ContactResolver,
        attachment_manager: &mut Option<AttachmentManager>,
        offset: i64,
    ) -> Result<SerializableMessage> {
        // Determine message type
        let message_type = if msg.is_tapback() {
            "tapback"
        } else if msg.is_edited() {
            "edited"
        } else if msg.is_announcement() {
            "announcement"
        } else {
            "normal"
        }
        .to_string();

        // Build metadata
        let metadata = MessageMetadata {
            rowid: msg.rowid,
            guid: msg.guid.clone(),
            date: format_timestamp(msg.date, offset),
            date_read: if msg.date_read > 0 {
                Some(format_timestamp(msg.date_read, offset))
            } else {
                None
            },
            date_delivered: if msg.date_delivered > 0 {
                Some(format_timestamp(msg.date_delivered, offset))
            } else {
                None
            },
            date_edited: if msg.date_edited > 0 {
                Some(format_timestamp(msg.date_edited, offset))
            } else {
                None
            },
            service: msg.service.as_deref().unwrap_or("Unknown").to_string(),
            is_from_me: msg.is_from_me,
            is_read: msg.is_read,
            chat_id: msg.chat_id,
            is_deleted: msg.deleted_from.is_some(),
        };

        // Build sender info
        let sender = self.build_sender(msg, handles, contact_resolver);

        // Build chat context
        let chat_context = self.build_chat_context(chat, handles);

        // Build content
        let content = self.build_content(db, msg, chat_id, attachment_manager)?;

        // Build relationships
        let relationships = SerializableRelationships {
            thread_originator_guid: msg.thread_originator_guid.clone(),
            thread_originator_part: msg.thread_originator_part.clone(),
            num_replies: msg.num_replies,
            tapbacks: vec![],   // TODO: Implement tapback resolution
            edit_history: None, // TODO: Implement edit history
        };

        // Build announcement metadata
        let announcement = if msg.is_announcement() {
            msg.get_announcement().and_then(|ann| match ann {
                Announcement::FullyUnsent => Some(SerializableAnnouncement::FullyUnsent),
                Announcement::GroupAction(action) => {
                    let serializable_action = match action {
                        GroupAction::ParticipantAdded(handle_id) => {
                            SerializableGroupAction::ParticipantAdded {
                                participant_handle_id: handle_id,
                            }
                        }
                        GroupAction::ParticipantRemoved(handle_id) => {
                            SerializableGroupAction::ParticipantRemoved {
                                participant_handle_id: handle_id,
                            }
                        }
                        GroupAction::NameChange(name) => SerializableGroupAction::NameChange {
                            new_name: name.to_string(),
                        },
                        GroupAction::ParticipantLeft => SerializableGroupAction::ParticipantLeft,
                        GroupAction::GroupIconChanged => SerializableGroupAction::GroupIconChanged,
                        GroupAction::GroupIconRemoved => SerializableGroupAction::GroupIconRemoved,
                        GroupAction::ChatBackgroundChanged => {
                            SerializableGroupAction::ChatBackgroundChanged
                        }
                        GroupAction::ChatBackgroundRemoved => {
                            SerializableGroupAction::ChatBackgroundRemoved
                        }
                    };
                    Some(SerializableAnnouncement::GroupAction(serializable_action))
                }
                Announcement::AudioMessageKept => Some(SerializableAnnouncement::AudioMessageKept),
                Announcement::Unknown(_) => None,
            })
        } else {
            None
        };

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
        handles: &HashMap<i32, Handle>,
        contact_resolver: &mut ContactResolver,
    ) -> SerializableSender {
        let (identifier, contact_name) = if let Some(handle_id) = msg.handle_id {
            if let Some(handle) = handles.get(&handle_id) {
                let id = handle.id.clone();
                let name = contact_resolver.resolve_name(&id, msg.is_from_me);
                (id, name)
            } else {
                ("Unknown".to_string(), None)
            }
        } else {
            let id = "Me".to_string();
            let name = contact_resolver.resolve_name(&id, true);
            (id, name)
        };

        SerializableSender {
            handle_id: msg.handle_id,
            identifier,
            contact_name,
        }
    }

    fn build_chat_context(
        &self,
        chat: &Chat,
        _handles: &HashMap<i32, Handle>,
    ) -> SerializableChatContext {
        // Get participants (simplified - just the chat identifier for now)
        let participants = vec![chat.chat_identifier.clone()];

        SerializableChatContext {
            chat_id: Some(chat.rowid),
            chat_identifier: chat.chat_identifier.clone(),
            display_name: chat.display_name().map(String::from),
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
        db: &Connection,
        msg: &Message,
        chat_id: i32,
        attachment_manager: &mut Option<AttachmentManager>,
    ) -> Result<SerializableContent> {
        let mut components = Vec::new();

        // Convert message components to serializable format
        for component in &msg.components {
            use imessage_database::tables::messages::models::BubbleComponent;

            match component {
                BubbleComponent::Text(attributes) => {
                    // Build text component with attributes
                    let text = msg.text.as_deref().unwrap_or("").to_string();
                    let attrs = attributes
                        .iter()
                        .map(|attr| {
                            let effects = attr
                                .effects
                                .iter()
                                .map(|effect| {
                                    use imessage_database::message_types::text_effects::TextEffect as DbEffect;
                                    match effect {
                                        DbEffect::Mention(id) => TextEffect::Mention {
                                            identifier: id.clone(),
                                        },
                                        DbEffect::Link(url) => TextEffect::Link { url: url.clone() },
                                        DbEffect::OTP => TextEffect::OTP,
                                        DbEffect::Conversion(_) => TextEffect::Conversion,
                                        _ => TextEffect::Default,
                                    }
                                })
                                .collect();

                            TextAttribute {
                                start: attr.start,
                                end: attr.end,
                                effects,
                            }
                        })
                        .collect();

                    components.push(ContentComponent::Text {
                        text,
                        attributes: attrs,
                    });
                }
                BubbleComponent::Attachment(meta) => {
                    // Query full attachment from database
                    let attachments = Attachment::from_message(db, msg)
                        .map_err(|e| anyhow::anyhow!("Failed to query attachment: {:?}", e))?;

                    // Find matching attachment by GUID
                    let attachment = attachments.into_iter().find(|att| {
                        meta.guid
                            .as_ref()
                            .map(|g| {
                                att.filename
                                    .as_ref()
                                    .map(|f| f.contains(g))
                                    .unwrap_or(false)
                            })
                            .unwrap_or(true)
                    });

                    if let Some(att) = attachment {
                        // Track converted MIME type (if conversion occurred)
                        let mut converted_mime_type: Option<String> = None;

                        // Handle attachment based on mode (embed or copy)
                        let (
                            copied_path,
                            copy_error,
                            embedded_data,
                            embedded_encoding,
                            embedded_compression,
                            content_hash,
                        ) = if self.embed_attachments {
                            // Embed mode
                            if let Some(ref mut mgr) = attachment_manager {
                                match mgr.embed_attachment(
                                    &att,
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
                        } else {
                            // Copy mode
                            if let Some(ref mut mgr) = attachment_manager {
                                match mgr.copy_attachment(&att, chat_id) {
                                    Ok((path, new_mime)) => {
                                        // Store converted MIME type if conversion occurred
                                        converted_mime_type = new_mime;
                                        (Some(path), None, None, None, None, None)
                                    }
                                    Err(err) => (None, Some(err), None, None, None, None),
                                }
                            } else {
                                (None, None, None, None, None, None)
                            }
                        };

                        // Use converted MIME type if available, otherwise use original
                        let final_mime_type = converted_mime_type.or_else(|| att.mime_type.clone());

                        // Build SerializableAttachment
                        let serializable = SerializableAttachment {
                            guid: meta.guid.clone(),
                            filename: att.filename.clone(),
                            transfer_name: att.transfer_name.clone(),
                            mime_type: final_mime_type,
                            uti: att.uti.clone(),
                            size_bytes: att.total_bytes,
                            transcription: meta.transcription.clone(),
                            dimensions: if meta.width.is_some() && meta.height.is_some() {
                                Some(AttachmentDimensions {
                                    width: meta.width.unwrap(),
                                    height: meta.height.unwrap(),
                                })
                            } else {
                                None
                            },
                            is_sticker: att.is_sticker,
                            sticker_metadata: None,
                            copied_path,
                            copy_error,
                            embedded_data,
                            embedded_encoding,
                            embedded_compression,
                            content_hash,
                        };

                        components.push(ContentComponent::Attachment(serializable));
                    } else {
                        // Attachment not found - include metadata with error
                        components.push(ContentComponent::Attachment(SerializableAttachment {
                            guid: meta.guid.clone(),
                            filename: None,
                            transfer_name: None,
                            mime_type: None,
                            uti: None,
                            size_bytes: 0,
                            transcription: meta.transcription.clone(),
                            dimensions: None,
                            is_sticker: false,
                            sticker_metadata: None,
                            copied_path: None,
                            copy_error: Some("Attachment not found in database".to_string()),
                            embedded_data: None,
                            embedded_encoding: None,
                            embedded_compression: None,
                            content_hash: None,
                        }));
                    }
                }
                BubbleComponent::App => {
                    // TODO: Implement app message conversion
                }
                BubbleComponent::Retracted => {
                    components.push(ContentComponent::Retracted);
                }
            }
        }

        Ok(SerializableContent {
            text: msg.text.clone(),
            subject: msg.subject.clone(),
            components,
        })
    }

    /// Write participants file for a chat
    fn write_participants_file(
        &self,
        chat_id: i32,
        handles: &HashMap<i32, Handle>,
        chatroom_participants: &HashMap<i32, BTreeSet<i32>>,
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
                    if let Some(ref mut mgr) = avatar_manager {
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

fn format_timestamp(timestamp: i64, _offset: i64) -> String {
    if timestamp == 0 {
        return String::new();
    }

    // Convert from Apple's epoch (2001-01-01) to Unix epoch (1970-01-01)
    // Apple epoch is 978307200 seconds after Unix epoch
    const COCOA_EPOCH_OFFSET: i64 = 978307200;
    let unix_timestamp = timestamp / 1_000_000_000 + COCOA_EPOCH_OFFSET;

    let datetime = chrono::DateTime::from_timestamp(unix_timestamp, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());

    datetime.format("%Y-%m-%dT%H:%M:%S%z").to_string()
}
