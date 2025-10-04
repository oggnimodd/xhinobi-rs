use clap::Parser;
use std::io::{self, BufRead, BufReader};
use std::fs;
use std::path::PathBuf;
use std::env;
use regex::Regex;

mod cli;
mod constants;
mod helpers;
mod decomment;
mod cache;

use cli::Args;
use constants::*;
use helpers::*;

#[derive(Debug)]
struct FileData {
    text: String,
    name: String,
}

fn get_files(files: &[String], args: &Args) -> Vec<FileData> {
    let mut results = Vec::new();
    
    'outer: for file in files {
        if file.is_empty() {
            continue;
        }
        
        // Check ignore patterns
        for pattern in &args.ignore {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(file) {
                    continue 'outer;
                }
            }
        }
        
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let file_path = current_dir.join(file);
        let basename = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file)
            .to_string();
        
        let mut file_content = if is_text_file(&basename) {
            match fs::read_to_string(&file_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error reading file {}: {}", file, e);
                    continue;
                }
            }
        } else {
            basename.clone()
        };

        if args.decomment {
            if let Some(language) = decomment::get_language(&file_path) {
                match decomment::clean_code(&file_content, language) {
                    Ok(cleaned_content) => file_content = cleaned_content,
                    Err(e) => eprintln!("Warning: Failed to decomment {}: {}", file, e),
                }
            }
        }
        
        results.push(FileData {
            text: file_content,
            name: format!("<{}>", basename),
        });
    }
    
    results
}

fn process_files(files: &[FileData], args: &Args) -> String {
    let mut final_output = String::new();

    // Add tree if requested
    if args.tree {
        final_output.push_str(&get_tree_output(&args.ignore));
    }

    // Process each file
    for file_data in files {
        if args.prepend_file_name {
            final_output.push_str(&format!("{} ", file_data.name));
        }
        final_output.push_str(&file_data.text);
    }

    // Minify if requested
    if args.minify {
        let re = Regex::new(r"\s+").unwrap();
        final_output = re.replace_all(&final_output, " ").trim().to_string();
    }

    final_output
}

fn output_to_clipboard(content: &str, args: &Args) {
    // Handle output based on environment and flags
    if args.osc52 {
        copy_to_clipboard_osc52(content);
        println!(
            "Sent {} characters (est. {} tokens) to clipboard via OSC52",
            content.len(),
            estimate_tokens(content)
        );
    } else if is_cloud_environment() {
        match create_temp_file(content) {
            Ok(temp_path) => {
                if let Err(e) = open_temp_file_in_code(&temp_path) {
                    eprintln!("Error opening temp file: {}", e);
                }
            }
            Err(e) => eprintln!("Error creating temp file: {}", e),
        }
    } else {
        match copy_to_clipboard(content) {
            Ok(method) => println!(
                "Copied {} characters (est. {} tokens) to clipboard using {}",
                content.len(),
                estimate_tokens(content),
                method
            ),
            Err(e) => {
                eprintln!("Clipboard copy failed: {}", e);
                println!("Printing content instead:");
                println!("{}", content);
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    // Handle cache-only operations
    if args.cache {
        match cache::load_most_recent_cache(&args.cache_dir) {
            Ok(entry) => {
                cache::copy_cache_to_clipboard(&entry, args.osc52).unwrap();
            }
            Err(e) => {
                eprintln!("Error loading cache: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if args.list_cache {
        match cache::interactive_cache_selection(&args.cache_dir, args.osc52) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error with cache selection: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if args.clear_cache {
        match cache::clear_cache(&args.cache_dir) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error clearing cache: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Read from stdin
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    let file_paths: Vec<String> = reader
        .lines()
        .map(|line| line.unwrap_or_default())
        .filter(|line| !line.is_empty())
        .collect();

    if !file_paths.is_empty() {
        let content = get_files(&file_paths, &args);
        let final_output = process_files(&content, &args);

        // Output to clipboard
        output_to_clipboard(&final_output, &args);

        // Save to cache (auto-save by default)
        let args_string = format!(
            "tree={} decomment={} minify={} prepend={} osc52={} ignore={}",
            args.tree,
            args.decomment,
            args.minify,
            args.prepend_file_name,
            args.osc52,
            args.ignore.join(",")
        );

        if let Err(e) = cache::save_to_cache(&final_output, content.len(), &args_string, &args.cache_dir) {
            eprintln!("Warning: Failed to save to cache: {}", e);
        }
    }
}