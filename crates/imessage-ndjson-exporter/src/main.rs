use anyhow::{Context, Result};
use clap::Parser;
use imessage_ndjson_core::{NdjsonExporter, attachment_manager::CompressionMode};
use std::path::PathBuf;

mod cli;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Validate CLI arguments before consuming common args
    cli.validate()
        .map_err(|e| anyhow::anyhow!("Invalid arguments: {}", e))?;

    // Extract values from common args before moving into librebar
    let verbose = cli.common.verbose;
    let quiet = cli.common.quiet;

    // Initialize librebar (logging to ~/Library/Logs/, crash handler).
    // _app holds logging/crash guards -- must stay alive for duration of main().
    let _app = librebar::init("imessage-ndjson-exporter")
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_cli(cli.common)
        .logging()
        .crash_handler()
        .start()
        .map_err(|e| anyhow::anyhow!("Failed to initialize: {}", e))?;

    // Determine database path
    let db_path = match cli.database_path.as_ref() {
        Some(path) => path.clone(),
        None => detect_database_path()?,
    };

    if verbose > 0 {
        println!("Database: {}", db_path.display());
        println!("Output: {}", cli.output_dir.display());
    }

    // Create output directory
    std::fs::create_dir_all(&cli.output_dir).context("Failed to create output directory")?;

    // Parse compression mode
    let embed_compression = CompressionMode::parse(&cli.embed_compression)
        .ok_or_else(|| anyhow::anyhow!("Invalid compression mode: {}", cli.embed_compression))?;

    // Show progress unless --quiet
    let show_progress = !quiet;

    // Create and run exporter
    let exporter = NdjsonExporter::new(
        &db_path,
        &cli.output_dir,
        cli.custom_name.clone(),
        show_progress,
        cli.conversation_filter.clone(),
        cli.contacts_path.clone(),
        cli.copy_attachments,
        cli.convert_attachments,
        cli.attachments_dir.clone(),
        cli.embed_attachments,
        cli.max_embed_size,
        embed_compression,
        cli.include_avatars,
        cli.start_date.clone(),
        cli.end_date.clone(),
    )?;

    exporter.export()?;

    if !quiet {
        println!("\nExport completed successfully!");
    }

    Ok(())
}

/// Attempt to auto-detect the iMessage database location
fn detect_database_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let default_path = PathBuf::from(home).join("Library/Messages/chat.db");

    if default_path.exists() {
        Ok(default_path)
    } else {
        anyhow::bail!("Could not auto-detect database location. Please specify with --database")
    }
}
