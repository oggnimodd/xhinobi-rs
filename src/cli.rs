use clap::Parser;

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
}
