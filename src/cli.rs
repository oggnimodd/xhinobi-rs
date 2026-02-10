use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone, Debug)]
pub enum TokenCounter {
    /// Approximate token count (1 token ~= 4 characters, +10% buffer).
    Estimate,
    /// Use tiktoken o200k_base encoding (OpenAI/GPT-4o/GPT-4.1/o1).
    #[value(name = "tiktoken-o200k")]
    TiktokenO200kBase,
    /// Approximate Gemini tokens by scaling tiktoken o200k_base counts.
    #[value(name = "gemini-approx")]
    GeminiApprox,
}

#[derive(Parser, Debug)]
#[command(name = "xhinobi")]
#[command(about = "A tool for aggregating text content from multiple files")]
#[command(version = "1.0")]
pub struct Args {
    /// Prepend the file name before the content
    #[arg(short = 'n', long = "prependFileName")]
    pub prepend_file_name: bool,
    
    /// Minify the output
    #[arg(short = 'm', long = "minify")]
    pub minify: bool,
    
    /// Glob patterns to ignore (can be used multiple times)
    #[arg(short = 'i', long = "ignore")]
    pub ignore: Vec<String>,
    
    /// Prepend the output with a directory tree (requires 'tree' command)
    #[arg(short = 't', long = "tree")]
    pub tree: bool,
    
    /// Use OSC52 escape sequence for clipboard over SSH
    #[arg(short = 'o', long = "osc52")]
    pub osc52: bool,

    /// Remove comments from files using tree-sitter
    #[arg(short = 'd', long = "decomment")]
    pub decomment: bool,

    /// Copy most recent cached result to clipboard (no stdin needed)
    #[arg(long = "cache")]
    pub cache: bool,

    /// Show interactive list of cached sessions
    #[arg(long = "list-cache")]
    pub list_cache: bool,

    /// Clear all cached sessions
    #[arg(long = "clear-cache")]
    pub clear_cache: bool,

    /// Override default cache directory
    #[arg(long = "cache-dir")]
    pub cache_dir: Option<String>,

    /// Show the cache directory path
    #[arg(long = "show-cache-dir")]
    pub show_cache_dir: bool,

    /// Token counting strategy
    #[arg(long = "token-counter", value_enum, default_value = "estimate")]
    pub token_counter: TokenCounter,

    /// Multiplier used for gemini-approx (default tuned to be close on large codebases)
    #[arg(long = "gemini-multiplier", default_value = "1.18")]
    pub gemini_multiplier: f64,

    /// Write output to a .txt file (will not overwrite existing files)
    #[arg(long = "output-file")]
    pub output_file: Option<String>,
}
