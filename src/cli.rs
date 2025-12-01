use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "imessage-ndjson-exporter")]
pub struct Cli {
    /// Path to the iMessage database (chat.db)
    ///
    /// If not specified, will attempt to auto-detect the database location
    #[arg(short = 'd', long = "database")]
    pub database_path: Option<PathBuf>,

    /// Output directory for NDJSON files
    ///
    /// One .ndjson file will be created per conversation
    #[arg(short = 'o', long = "output", required = true)]
    pub output_dir: PathBuf,

    /// Start date filter (YYYY-MM-DD format)
    #[arg(long = "start-date")]
    pub start_date: Option<String>,

    /// End date filter (YYYY-MM-DD format)
    #[arg(long = "end-date")]
    pub end_date: Option<String>,

    /// Filter by specific chat IDs (comma-separated)
    #[arg(long = "chat-ids")]
    pub chat_ids: Option<String>,

    /// Filter by specific handle IDs (comma-separated)
    #[arg(long = "handle-ids")]
    pub handle_ids: Option<String>,

    /// Filter conversations by contact names, phone numbers, or emails
    ///
    /// Comma-separated list. All conversations with any matching participant will be exported.
    /// Example: -t "steve@apple.com,Jane Doe,5558675309"
    #[arg(short = 't', long = "conversation-filter")]
    pub conversation_filter: Option<String>,

    /// Path to contacts database (optional, auto-detected if not specified)
    ///
    /// By default, scans ~/Library/Application Support/AddressBook/Sources/*/AddressBook-v22.abcddb
    #[arg(long = "contacts-path")]
    pub contacts_path: Option<PathBuf>,

    /// Copy attachments to output directory
    #[arg(long = "copy-attachments")]
    pub copy_attachments: bool,

    /// Convert attachments to compatible formats (requires --copy-attachments)
    ///
    /// Converts HEIC to JPEG, MOV to MP4, etc. Currently stubbed - copies raw files.
    #[arg(long = "convert-attachments")]
    pub convert_attachments: bool,

    /// Custom directory name for attachments (default: "attachments")
    #[arg(long = "attachments-dir", default_value = "attachments")]
    pub attachments_dir: String,

    /// Embed attachments directly in JSON output (mutually exclusive with --copy-attachments)
    ///
    /// Attachments are base64-encoded and included in the message JSON.
    /// Makes exports more portable but increases file size significantly.
    #[arg(long = "embed-attachments")]
    pub embed_attachments: bool,

    /// Maximum attachment size for embedding in bytes (default: 10MB)
    ///
    /// Attachments larger than this will be skipped with an error.
    /// Only valid with --embed-attachments.
    #[arg(long = "max-embed-size", default_value = "10485760")]
    pub max_embed_size: usize,

    /// Compression method for embedded attachments: auto, gzip, zstd, none
    ///
    /// 'auto' intelligently skips compression for already-compressed formats (JPEG, MP4, etc.)
    /// and uses zstd for everything else. Only valid with --embed-attachments.
    #[arg(long = "embed-compression", default_value = "auto")]
    pub embed_compression: String,

    /// Custom name for the database owner (overrides contact resolution)
    #[arg(long = "custom-name")]
    pub custom_name: Option<String>,

    /// Enable debug logging
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Disable progress indicators
    #[arg(long = "no-progress")]
    pub no_progress: bool,
}

impl Cli {
    /// Parse chat IDs from comma-separated string
    pub fn parse_chat_ids(&self) -> Option<Vec<i32>> {
        self.chat_ids.as_ref().and_then(|ids| {
            ids.split(',')
                .map(|id| id.trim().parse::<i32>().ok())
                .collect::<Option<Vec<i32>>>()
        })
    }

    /// Parse handle IDs from comma-separated string
    pub fn parse_handle_ids(&self) -> Option<Vec<i32>> {
        self.handle_ids.as_ref().and_then(|ids| {
            ids.split(',')
                .map(|id| id.trim().parse::<i32>().ok())
                .collect::<Option<Vec<i32>>>()
        })
    }

    /// Validate CLI arguments
    pub fn validate(&self) -> Result<(), String> {
        // --convert-attachments requires --copy-attachments
        if self.convert_attachments && !self.copy_attachments {
            return Err("--convert-attachments requires --copy-attachments".to_string());
        }

        // --embed-attachments and --copy-attachments are mutually exclusive
        if self.embed_attachments && self.copy_attachments {
            return Err(
                "--embed-attachments and --copy-attachments are mutually exclusive".to_string(),
            );
        }

        // Validate embed-compression value
        if self.embed_attachments {
            let valid_compression = ["auto", "gzip", "zstd", "none"];
            if !valid_compression.contains(&self.embed_compression.as_str()) {
                return Err(format!(
                    "--embed-compression must be one of: {}",
                    valid_compression.join(", ")
                ));
            }
        }

        Ok(())
    }
}
