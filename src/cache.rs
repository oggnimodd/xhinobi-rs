use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Context, Result};
use inquire::Select;

use crate::helpers::{copy_to_clipboard_osc52, copy_to_clipboard};
use colored::Colorize;
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
    #[serde(default)]
    pub token_counter: Option<String>,
    pub file_size: usize,
    pub source_file_count: usize,
    pub args_used: String,
    pub working_dir: String,
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
    #[serde(default)]
    pub token_counter: Option<String>,
    pub file_size: usize,
    pub source_file_count: usize,
    pub args_used: String,
    pub working_dir: String,
}

fn token_prefix(entry: &CacheEntry) -> &'static str {
    match entry.token_counter.as_deref() {
        Some("tiktoken-o200k") => "",
        Some("gemini-approx") => "",
        Some("estimate") | None => "est. ",
        _ => "",
    }
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
    token_count: usize,
    token_counter: Option<String>,
) -> Result<()> {
    let cache_dir = get_cache_dir(cache_dir_override)?;
    let sessions_dir = cache_dir.join("sessions");

    // Create timestamped filename
    let timestamp = Utc::now();
    let filename = format!("{}.cache", timestamp.format("%Y-%m-%d_%H-%M-%S"));
    let file_path = sessions_dir.join(&filename);

    // Create cache entry
    let working_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        .to_string_lossy()
        .to_string();

    let entry = CacheEntry {
        timestamp,
        content: content.to_string(),
        token_count,
        token_counter,
        file_size: content.len(),
        source_file_count,
        args_used: args_used.to_string(),
        working_dir,
    };

    // Save cache entry
    let serialized = serde_json::to_string(&entry).context("Failed to serialize cache entry")?;
    fs::write(&file_path, serialized).context("Failed to write cache file")?;

    // Update index
    update_cache_index(&cache_dir, &entry, &filename)?;

    // Cleanup old entries if needed
    cleanup_cache(&cache_dir)?;

    println!(
        "Cached result ({} characters, {}{} tokens)",
        content.len(),
        token_prefix(&entry),
        entry.token_count.to_string().cyan()
    );

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
    let mut index: CacheIndex = serde_json::from_str(&index_content).context("Failed to parse cache index")?;

    // Sort entries by timestamp descending (newest first)
    index.entries.sort_by_key(|e| std::cmp::Reverse(e.timestamp));

    Ok(index.entries)
}

pub fn copy_cache_to_clipboard(entry: &CacheEntry, osc52: bool) -> Result<()> {
    if osc52 {
        copy_to_clipboard_osc52(&entry.content);
        println!(
            "Copied {} characters ({}{} tokens) to clipboard via OSC52",
            entry.content.len(),
            token_prefix(entry),
            entry.token_count.to_string().cyan()
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
                "Copied {} characters ({}{} tokens) to clipboard using {}",
                entry.content.len(),
                token_prefix(entry),
                entry.token_count.to_string().cyan(),
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
        token_counter: entry.token_counter.clone(),
        file_size: entry.file_size,
        source_file_count: entry.source_file_count,
        args_used: entry.args_used.clone(),
        working_dir: entry.working_dir.clone(),
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

    // Create selection options with simplified info
    let options: Vec<String> = entries.iter()
        .enumerate()
        .map(|(i, entry)| {
            let local_time = entry.timestamp.with_timezone(&chrono::Local);
            // Use home directory replacement for cleaner paths
            let working_dir = entry.working_dir.replace(&env::var("HOME").unwrap_or_default(), "~");
            format!(
                "[{:02}] {} | {} chars | {} tokens | {} files | {}",
                i + 1,
                local_time.format("%d %b %Y %H:%M"),
                entry.file_size,
                entry.token_count.to_string().cyan(),
                entry.source_file_count,
                working_dir
            )
        })
        .collect();

    let selected = Select::new("Select a cache entry to copy to clipboard:", options.clone())
        .with_page_size(10)
        .prompt();

    match selected {
        Ok(choice) => {
            println!(); // Add a blank line for better formatting

            // Find the selected entry by extracting the index from the choice string
            let selected_index = choice
                .split(']')
                .next()
                .and_then(|s| s.trim_start_matches('[').trim().parse::<usize>().ok())
                .map(|i| i - 1) // Convert 1-based to 0-based index
                .ok_or_else(|| anyhow::anyhow!("Could not parse selection index"))?;

            if let Some(selected_entry) = entries.get(selected_index) {
                let cache_dir = get_cache_dir(cache_dir_override)?;
                let cache_file_path = cache_dir.join("sessions").join(&selected_entry.filename);
                let cache_content = fs::read_to_string(&cache_file_path).context("Failed to read cache file")?;
                let entry: CacheEntry = serde_json::from_str(&cache_content).context("Failed to parse cache entry")?;

                copy_cache_to_clipboard(&entry, osc52)?;
                println!("✓ Selected cache entry copied to clipboard!");
            }
        }
        Err(inquire::InquireError::OperationCanceled) => {
            println!("Selection cancelled.");
        }
        Err(inquire::InquireError::OperationInterrupted) => {
            println!("Operation interrupted.");
        }
        Err(e) => {
            eprintln!("Error during selection: {}", e);
        }
    }

    Ok(())
}
