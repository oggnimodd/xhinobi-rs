use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use std::io::{self, Read};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Context, Result};

use crate::helpers::{copy_to_clipboard_osc52, estimate_tokens, copy_to_clipboard};
use crate::constants::is_cloud_environment;

const MAX_CACHE_ENTRIES: usize = 50;
const MAX_CACHE_SIZE_MB: u64 = 100;
const MAX_CACHE_AGE_DAYS: i64 = 90;
const CACHE_DIR_NAME: &str = "xhinobi";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub token_count: usize,
    pub file_size: usize,
    pub source_file_count: usize,
    pub args_used: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheIndex {
    pub entries: Vec<CacheIndexEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheIndexEntry {
    pub filename: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: usize,
    pub file_size: usize,
    pub source_file_count: usize,
    pub args_used: String,
}

pub fn get_cache_dir(override_dir: &Option<String>) -> Result<PathBuf> {
    let cache_dir = if let Some(custom_dir) = override_dir {
        PathBuf::from(custom_dir)
    } else {
        // Use XDG cache directory or fallback to ~/.cache
        let xdg_cache_home = env::var("XDG_CACHE_HOME").ok();
        if let Some(xdg_dir) = xdg_cache_home {
            PathBuf::from(xdg_dir).join(CACHE_DIR_NAME)
        } else {
            let home = env::var("HOME").context("Could not find HOME directory")?;
            PathBuf::from(home).join(".cache").join(CACHE_DIR_NAME)
        }
    };

    let sessions_dir = cache_dir.join("sessions");
    fs::create_dir_all(&sessions_dir).context("Failed to create cache directory")?;

    Ok(cache_dir)
}

pub fn save_to_cache(
    content: &str,
    source_file_count: usize,
    args_used: &str,
    cache_dir_override: &Option<String>,
) -> Result<()> {
    let cache_dir = get_cache_dir(cache_dir_override)?;
    let sessions_dir = cache_dir.join("sessions");

    // Create timestamped filename
    let timestamp = Utc::now();
    let filename = format!("{}.cache", timestamp.format("%Y-%m-%d_%H-%M-%S"));
    let file_path = sessions_dir.join(&filename);

    // Create cache entry
    let entry = CacheEntry {
        timestamp,
        content: content.to_string(),
        token_count: estimate_tokens(content),
        file_size: content.len(),
        source_file_count,
        args_used: args_used.to_string(),
    };

    // Save cache entry
    let serialized = serde_json::to_string(&entry).context("Failed to serialize cache entry")?;
    fs::write(&file_path, serialized).context("Failed to write cache file")?;

    // Update index
    update_cache_index(&cache_dir, &entry, &filename)?;

    // Cleanup old entries if needed
    cleanup_cache(&cache_dir)?;

    println!("Cached result ({} characters, {} tokens)", content.len(), entry.token_count);

    Ok(())
}

pub fn load_most_recent_cache(cache_dir_override: &Option<String>) -> Result<CacheEntry> {
    let cache_dir = get_cache_dir(cache_dir_override)?;
    let index_path = cache_dir.join("sessions").join("cache_index.json");

    if !index_path.exists() {
        return Err(anyhow::anyhow!("No cache found"));
    }

    let index_content = fs::read_to_string(&index_path).context("Failed to read cache index")?;
    let index: CacheIndex = serde_json::from_str(&index_content).context("Failed to parse cache index")?;

    if index.entries.is_empty() {
        return Err(anyhow::anyhow!("No cache entries found"));
    }

    // Find most recent entry
    let most_recent = index.entries.iter()
        .max_by_key(|e| e.timestamp)
        .ok_or_else(|| anyhow::anyhow!("No cache entries found"))?;

    let cache_file_path = cache_dir.join("sessions").join(&most_recent.filename);
    let cache_content = fs::read_to_string(&cache_file_path).context("Failed to read cache file")?;
    let entry: CacheEntry = serde_json::from_str(&cache_content).context("Failed to parse cache entry")?;

    Ok(entry)
}

pub fn list_cache_entries(cache_dir_override: &Option<String>) -> Result<Vec<CacheIndexEntry>> {
    let cache_dir = get_cache_dir(cache_dir_override)?;
    let index_path = cache_dir.join("sessions").join("cache_index.json");

    if !index_path.exists() {
        return Ok(vec![]);
    }

    let index_content = fs::read_to_string(&index_path).context("Failed to read cache index")?;
    let index: CacheIndex = serde_json::from_str(&index_content).context("Failed to parse cache index")?;

    Ok(index.entries)
}

pub fn copy_cache_to_clipboard(entry: &CacheEntry, osc52: bool) -> Result<()> {
    if osc52 {
        copy_to_clipboard_osc52(&entry.content);
        println!(
            "Copied {} characters (est. {} tokens) to clipboard via OSC52",
            entry.content.len(),
            entry.token_count
        );
    } else if is_cloud_environment() {
        use crate::helpers::{create_temp_file, open_temp_file_in_code};

        match create_temp_file(&entry.content) {
            Ok(temp_path) => {
                if let Err(e) = open_temp_file_in_code(&temp_path) {
                    eprintln!("Error opening temp file: {}", e);
                }
            }
            Err(e) => eprintln!("Error creating temp file: {}", e),
        }
    } else {
        match copy_to_clipboard(&entry.content) {
            Ok(method) => println!(
                "Copied {} characters (est. {} tokens) to clipboard using {}",
                entry.content.len(),
                entry.token_count,
                method
            ),
            Err(e) => {
                eprintln!("Clipboard copy failed: {}", e);
                println!("Printing content instead:");
                println!("{}", entry.content);
            }
        }
    }

    Ok(())
}

pub fn clear_cache(cache_dir_override: &Option<String>) -> Result<()> {
    let cache_dir = get_cache_dir(cache_dir_override)?;
    let sessions_dir = cache_dir.join("sessions");

    if sessions_dir.exists() {
        fs::remove_dir_all(&sessions_dir).context("Failed to remove cache directory")?;
        fs::create_dir(&sessions_dir).context("Failed to recreate cache directory")?;
        println!("Cache cleared successfully");
    } else {
        println!("No cache directory found");
    }

    Ok(())
}

fn update_cache_index(cache_dir: &Path, entry: &CacheEntry, filename: &str) -> Result<()> {
    let index_path = cache_dir.join("sessions").join("cache_index.json");

    let mut index = if index_path.exists() {
        let index_content = fs::read_to_string(&index_path).context("Failed to read existing index")?;
        serde_json::from_str(&index_content).context("Failed to parse existing index")?
    } else {
        CacheIndex { entries: vec![] }
    };

    let index_entry = CacheIndexEntry {
        filename: filename.to_string(),
        timestamp: entry.timestamp,
        token_count: entry.token_count,
        file_size: entry.file_size,
        source_file_count: entry.source_file_count,
        args_used: entry.args_used.clone(),
    };

    index.entries.push(index_entry);

    let serialized = serde_json::to_string(&index).context("Failed to serialize updated index")?;
    fs::write(&index_path, serialized).context("Failed to write updated index")?;

    Ok(())
}

fn cleanup_cache(cache_dir: &Path) -> Result<()> {
    let sessions_dir = cache_dir.join("sessions");
    let index_path = sessions_dir.join("cache_index.json");

    if !index_path.exists() {
        return Ok(());
    }

    let index_content = fs::read_to_string(&index_path).context("Failed to read index for cleanup")?;
    let mut index: CacheIndex = serde_json::from_str(&index_content).context("Failed to parse index for cleanup")?;

    // Remove entries older than MAX_CACHE_AGE_DAYS
    let cutoff_date = Utc::now() - chrono::Duration::days(MAX_CACHE_AGE_DAYS);
    index.entries.retain(|e| e.timestamp > cutoff_date);

    // Limit by number of entries
    if index.entries.len() > MAX_CACHE_ENTRIES {
        index.entries.sort_by_key(|e| e.timestamp);
        index.entries.truncate(MAX_CACHE_ENTRIES);
        index.entries.sort_by_key(|e| std::cmp::Reverse(e.timestamp)); // Sort back to newest first
    }

    // Calculate total size and remove oldest if exceeding size limit
    let mut total_size: u64 = index.entries.iter().map(|e| e.file_size as u64).sum();
    let max_size_bytes = MAX_CACHE_SIZE_MB * 1024 * 1024;

    if total_size > max_size_bytes {
        index.entries.sort_by_key(|e| e.timestamp);
        while total_size > max_size_bytes && !index.entries.is_empty() {
            let oldest_filename = if let Some(oldest) = index.entries.first() {
                total_size -= oldest.file_size as u64;
                oldest.filename.clone()
            } else {
                break;
            };

            index.entries.remove(0);

            // Remove actual cache file
            let cache_file = sessions_dir.join(&oldest_filename);
            let _ = fs::remove_file(cache_file);
        }
        index.entries.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
    }

    // Remove cache files that are no longer in index
    let cached_files: std::collections::HashSet<String> = index.entries.iter()
        .map(|e| e.filename.clone())
        .collect();

    if let Ok(entries) = fs::read_dir(&sessions_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".cache") && !cached_files.contains(filename) {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }

    // Write updated index
    let serialized = serde_json::to_string(&index).context("Failed to serialize cleaned index")?;
    fs::write(&index_path, serialized).context("Failed to write cleaned index")?;

    Ok(())
}

pub fn interactive_cache_selection(cache_dir_override: &Option<String>, osc52: bool) -> Result<()> {
    let entries = list_cache_entries(cache_dir_override)?;

    if entries.is_empty() {
        println!("No cached entries found.");
        return Ok(());
    }

    println!("Cache Entries (use arrow keys to navigate, Enter to select, Esc to quit):");
    println!();

    let mut selected_index = 0;

    loop {
        // Clear screen and redraw
        print!("\x1B[2J\x1B[1;1H");
        println!("Cache Entries (use arrow keys to navigate, Enter to select, Esc to quit):");
        println!();

        for (i, entry) in entries.iter().enumerate() {
            let marker = if i == selected_index { "►" } else { " " };
            let local_time = entry.timestamp.with_timezone(&chrono::Local);
            println!(
                "{} [{:02}] {} | {} chars | {} tokens | {} files | {}",
                marker,
                i + 1,
                local_time.format("%Y-%m-%d %H:%M:%S"),
                entry.file_size,
                entry.token_count,
                entry.source_file_count,
                entry.args_used
            );
        }

        // Read single key press
        let mut buffer = [0u8; 1];
        if io::stdin().read_exact(&mut buffer).is_ok() {
            match buffer[0] {
                b'\x1B' => { // Esc sequence
                    // Check if it's a proper escape (just ESC) or arrow key sequence
                    let mut seq_buffer = [0u8; 2];
                    if io::stdin().read_exact(&mut seq_buffer).is_ok() {
                        if seq_buffer[0] == b'[' {
                            match seq_buffer[1] {
                                b'A' => { // Up arrow
                                    if selected_index > 0 {
                                        selected_index -= 1;
                                    }
                                }
                                b'B' => { // Down arrow
                                    if selected_index < entries.len() - 1 {
                                        selected_index += 1;
                                    }
                                }
                                _ => {}
                            }
                        }
                    } else {
                        // Just ESC key pressed
                        println!("\nCancelled selection.");
                        return Ok(());
                    }
                }
                b'\r' | b'\n' => { // Enter
                    if let Some(selected_entry) = entries.get(selected_index) {
                        let cache_dir = get_cache_dir(cache_dir_override)?;
                        let cache_file_path = cache_dir.join("sessions").join(&selected_entry.filename);
                        let cache_content = fs::read_to_string(&cache_file_path).context("Failed to read cache file")?;
                        let entry: CacheEntry = serde_json::from_str(&cache_content).context("Failed to parse cache entry")?;

                        copy_cache_to_clipboard(&entry, osc52)?;
                        println!("\nSelected cache entry copied to clipboard!");
                        return Ok(());
                    }
                }
                b'q' | b'Q' => {
                    println!("\nCancelled selection.");
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}