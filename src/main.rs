use clap::Parser;
use std::io::{self, BufRead, BufReader, Write};
use std::fs;
use std::path::PathBuf;
use std::env;
use std::process::{Command, Stdio};
use regex::Regex;

mod cli;
mod constants;
mod helpers;
mod decomment;

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

fn process_files(files: &[FileData], args: &Args) {
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
    
    // Handle output based on environment and flags
    if args.osc52 {
        copy_to_clipboard_osc52(&final_output);
        println!(
            "Sent {} characters (est. {} tokens) to clipboard via OSC52",
            final_output.len(),
            estimate_tokens(&final_output)
        );
    } else if is_cloud_environment() {
        match create_temp_file(&final_output) {
            Ok(temp_path) => {
                if let Err(e) = open_temp_file_in_code(&temp_path) {
                    eprintln!("Error opening temp file: {}", e);
                }
            }
            Err(e) => eprintln!("Error creating temp file: {}", e),
        }
    } else {
        match copy_to_clipboard(&final_output) {
            Ok(method) => println!(
                "Copied {} characters (est. {} tokens) to clipboard using {}",
                final_output.len(),
                estimate_tokens(&final_output),
                method
            ),
            Err(e) => {
                eprintln!("Clipboard copy failed: {}", e);
                println!("Printing content instead:");
                println!("{}", final_output);
            }
        }
    }
}

fn copy_to_clipboard(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Check if xclip is installed by checking its version.
    if Command::new("xclip").arg("-version").output().is_err() {
        return Err("xclip command not found. Please install it to use the clipboard.".into());
    }

    let mut child = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(Stdio::piped())
        .spawn()?;

    // Pipe the text to the xclip process.
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    } else {
        // This case is unlikely but handled for robustness.
        return Err("Failed to open stdin for the xclip process.".into());
    }

    // Wait for the xclip process to complete.
    let status = child.wait()?;
    if status.success() {
        Ok("xclip".to_string())
    } else {
        Err(format!("xclip process exited with status: {}", status).into())
    }
}

fn main() {
    let args = Args::parse();
    
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
        process_files(&content, &args);
    }
}