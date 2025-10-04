use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::env;
use std::io::Write;
use base64::{Engine as _, engine::general_purpose};
use crate::constants::*;

pub fn is_text_file(filename: &str) -> bool {
    let text_extensions = vec![
        "dockerfile", "makefile", "rakefile", "ada", "adb", "ads", "applescript", "as", "ascx", 
        "asm", "asmx", "asp", "aspx", "atom", "bas", "bash", "bashrc", "bat", "bbcolors", 
        "bdsgroup", "bdsproj", "bib", "bowerrc", "c", "cbl", "cc", "cfc", "cfg", "cfm", 
        "cfml", "cgi", "clj", "cls", "cmake", "cmd", "cnf", "cob", "coffee", "coffeekup", 
        "conf", "cpp", "cpt", "cpy", "crt", "cs", "csh", "cson", "csr", "css", "csslintrc", 
        "csv", "ctl", "curlrc", "cxx", "dart", "dfm", "diff", "dockerignore", "dof", "dpk", 
        "dproj", "dtd", "eco", "editorconfig", "ejs", "el", "emacs", "eml", "ent", "erb", 
        "erl", "eslintignore", "eslintrc", "ex", "exs", "f", "f03", "f77", "f90", "f95", 
        "fish", "for", "fpp", "frm", "ftn", "gemrc", "gitattributes", "gitconfig", 
        "gitignore", "gitkeep", "gitmodules", "go", "gpp", "gradle", "groovy", "groupproj", 
        "grunit", "gtmpl", "gvimrc", "h", "haml", "hbs", "hgignore", "hh", "hpp", "hrl", 
        "hs", "hta", "htaccess", "htc", "htm", "html", "htpasswd", "hxx", "iced", "inc", 
        "ini", "ino", "int", "irbrc", "itcl", "itermcolors", "itk", "jade", "java", "jhtm", 
        "jhtml", "js", "jscsrc", "jshintignore", "jshintrc", "json", "json5", "jsonld", 
        "jsp", "jspx", "jsx", "ksh", "kt", "less", "lhs", "lisp", "log", "ls", "lsp", 
        "lua", "m", "mak", "map", "markdown", "master", "md", "mdown", "mdwn", "mdx", 
        "metadata", "mht", "mhtml", "mjs", "mk", "mkd", "mkdn", "mkdown", "ml", "mli", 
        "mm", "mxml", "nfm", "nfo", "njk", "noon", "npmignore", "npmrc", "nvmrc", "ops", 
        "pas", "pasm", "patch", "pbxproj", "pch", "pem", "pg", "php", "php3", "php4", 
        "php5", "phpt", "phtml", "pir", "pl", "pm", "pmc", "pod", "pot", "prisma", 
        "properties", "props", "pt", "pug", "py", "r", "rake", "rb", "rdoc", "rdoc_options", 
        "resx", "rhtml", "rjs", "rlib", "rmd", "ron", "rs", "rss", "rst", "rtf", "rvmrc", 
        "rxml", "s", "sass", "scala", "scm", "scss", "seestyle", "sh", "shtml", "sls", 
        "spec", "sql", "sqlite", "ss", "sss", "st", "strings", "sty", "styl", "stylus", 
        "sub", "sublime-build", "sublime-commands", "sublime-completions", "sublime-keymap", 
        "sublime-macro", "sublime-menu", "sublime-project", "sublime-settings", 
        "sublime-workspace", "sv", "svelte", "svc", "svg", "t", "tcl", "tcsh", "terminal", 
        "tex", "text", "textile", "tg", "tmlanguage", "tmtheme", "tmpl", "toml", "tpl", 
        "ts", "tsv", "tsx", "tt", "tt2", "ttml", "txt", "v", "vb", "vbs", "vh", "vhd", 
        "vhdl", "vim", "viminfo", "vimrc", "vue", "webapp", "wxml", "wxss", "x-php", 
        "xaml", "xht", "xhtml", "xml", "xs", "xsd", "xsl", "xslt", "yaml", "yml", "zsh", 
        "zshrc"
    ];
    
    let lower_filename = filename.to_lowercase();
    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // Check exact filename matches
    if text_extensions.contains(&lower_filename.as_str()) {
        return true;
    }
    
    // Check extension matches
    if !extension.is_empty() && text_extensions.contains(&extension.as_str()) {
        return true;
    }
    
    false
}

pub fn get_tree_output(ignore_patterns: &[String]) -> String {
    // Check if tree command exists
    if Command::new("tree").arg("--version").output().is_err() {
        eprintln!("Warning: 'tree' command not found. Skipping tree generation.");
        return String::new();
    }
    
    let mut args = vec!["-I", "node_modules|dist|vendor|*.log|tmp|images|go.sum|*.lock"];
    
    // Add ignore patterns
    for pattern in ignore_patterns {
        args.push("-I");
        args.push(pattern);
    }
    
    match Command::new("tree").args(&args).output() {
        Ok(output) => {
            let tree_output = String::from_utf8_lossy(&output.stdout);
            format!("--- FOLDER TREE ---\n{}\n--- FILE CONTENT ---\n\n", tree_output)
        }
        Err(e) => {
            eprintln!("Warning: 'tree' command finished with an error: {}", e);
            String::new()
        }
    }
}

pub fn create_temp_file(content: &str) -> Result<PathBuf, std::io::Error> {
    let temp_path = if is_cloud_environment() {
        env::var("HOME")
            .map(|home| PathBuf::from(home).join(TEMP_FILE_NAME))
            .unwrap_or_else(|_| PathBuf::from(TEMP_FILE_NAME))
    } else {
        env::temp_dir().join(TEMP_FILE_NAME)
    };
    
    // Remove existing temp file if it exists
    if temp_path.exists() {
        fs::remove_file(&temp_path)?;
    }
    
    fs::write(&temp_path, content)?;
    Ok(temp_path)
}

pub fn open_temp_file_in_code(file_path: &Path) -> Result<(), std::io::Error> {
    let command = if is_google_cloud() {
        Command::new("cloudshell")
            .arg("open")
            .arg(file_path)
            .spawn()?
    } else {
        Command::new("code")
            .arg(file_path)
            .spawn()?
    };
    
    let _ = command.wait_with_output();
    Ok(())
}

pub fn copy_to_clipboard_osc52(text: &str) {
    let encoded = general_purpose::STANDARD.encode(text.as_bytes());
    print!("\x1b]52;c;{}\x07", encoded);
}

/// Estimates the number of tokens based on the rule of thumb that 1 token is ~4 characters.
pub fn estimate_tokens(text: &str) -> usize {
    ((text.len() as f64 / 4.0) * 1.1).ceil() as usize
}

pub fn copy_to_clipboard(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let is_wayland = env::var("WAYLAND_DISPLAY").is_ok();

    if is_wayland {
        if Command::new("wl-copy").arg("--version").output().is_ok() {
            let mut child = Command::new("wl-copy")
                .stdin(Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            } else {
                return Err("Failed to open stdin for the wl-copy process.".into());
            }
            let status = child.wait()?;
            if status.success() {
                return Ok("wl-copy".to_string());
            } else {
                return Err(format!("wl-copy process exited with status: {}", status).into());
            }
        }
    }

    // Fallback to xclip
    if Command::new("xclip").arg("-version").output().is_err() {
        return Err("xclip command not found. Please install it to use the clipboard.".into());
    }

    let mut child = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    } else {
        return Err("Failed to open stdin for the xclip process.".into());
    }

    let status = child.wait()?;
    if status.success() {
        Ok("xclip".to_string())
    } else {
        Err(format!("xclip process exited with status: {}", status).into())
    }
}
