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
}