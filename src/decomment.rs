use anyhow::{Context, Result};
use std::path::Path;
use tree_sitter::{Language, Parser, Range};

pub fn clean_code(content: &str, language: Language) -> Result<String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .context("Error loading language grammar")?;
    let tree = parser
        .parse(content, None)
        .context("Failed to parse the code")?;

    let mut comments_to_remove: Vec<Range> = Vec::new();
    let mut cursor = tree.root_node().walk();

    loop {
        let node = cursor.node();
        if node.kind().contains("comment") {
            comments_to_remove.push(node.range());
        }

        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        loop {
            if !cursor.goto_parent() {
                break;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
        if cursor.node() == tree.root_node() && !cursor.goto_next_sibling() {
            break;
        }
    }

    comments_to_remove.sort_by_key(|r| r.start_byte);

    let mut merged_ranges: Vec<Range> = Vec::new();
    if !comments_to_remove.is_empty() {
        let mut current_range = comments_to_remove[0];
        for &next_range in &comments_to_remove[1..] {
            if next_range.start_byte < current_range.end_byte {
                current_range.end_byte = current_range.end_byte.max(next_range.end_byte);
            } else {
                merged_ranges.push(current_range);
                current_range = next_range;
            }
        }
        merged_ranges.push(current_range);
    }

    let mut content_without_comments = String::new();
    let mut current_byte_pos = 0;

    for range in merged_ranges {
        content_without_comments.push_str(&content[current_byte_pos..range.start_byte]);
        current_byte_pos = range.end_byte;
    }
    content_without_comments.push_str(&content[current_byte_pos..]);

    let final_cleaned_content: String = content_without_comments
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim_end())
        .collect::<Vec<&str>>()
        .join("\n");

    Ok(final_cleaned_content)
}

pub fn get_language(file_path: &Path) -> Option<Language> {
    let extension = file_path.extension()?.to_str()?;
    match extension {
        "ts" => Some(tree_sitter_typescript::language_typescript()),
        "tsx" => Some(tree_sitter_typescript::language_tsx()),
        "js" | "jsx" | "mjs" => Some(tree_sitter_javascript::language()),
        "json" => Some(tree_sitter_json::language()),
        "py" => Some(tree_sitter_python::language()),
        "rs" => Some(tree_sitter_rust::language()),
        "go" => Some(tree_sitter_go::language()),
        "sh" | "bash" => Some(tree_sitter_bash::language()),
        "php" => Some(tree_sitter_php::language_php()),
        "lua" => Some(tree_sitter_lua::language()),
        _ => None,
    }
}