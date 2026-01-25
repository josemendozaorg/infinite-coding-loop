use std::process::Command;
use walkdir::WalkDir;

pub struct ContextEnricher;

impl ContextEnricher {
    pub fn new() -> Self {
        Self
    }

    pub fn get_file_tree(&self, path: &str, max_depth: usize) -> String {
        let mut tree = String::new();
        // Check if path exists first
        if !std::path::Path::new(path).exists() {
            return "Path does not exist.".to_string();
        }

        for entry in WalkDir::new(path)
            .max_depth(max_depth)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let depth = entry.depth();
            // Skip hidden files/dirs for noise reduction (basic check)
            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.') && name != "." {
                if entry.file_type().is_dir() {
                    // We can't easily skip descent here with filter_map iterator style easily
                    // without using entry.into_iter().filter_entry...
                    // For now, just don't print them, but walkdir will still visit children.
                    // This is 'basic' implementation.
                    continue;
                }
                continue;
            }

            let indent = "  ".repeat(depth);
            if depth > 0 {
                // Skip root
                let marker = if entry.file_type().is_dir() { "/" } else { "" };
                tree.push_str(&format!("{}{}{}\n", indent, name, marker));
            }
        }
        if tree.is_empty() {
            return "(Empty directory or no visible files)".to_string();
        }
        tree
    }

    pub fn get_git_status(&self, path: &str) -> String {
        let output = Command::new("git")
            .args(["status", "--short"])
            .current_dir(path)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if s.is_empty() {
                        "(Clean)".to_string()
                    } else {
                        s
                    }
                } else {
                    format!("Git Error: {}", String::from_utf8_lossy(&out.stderr))
                }
            }
            Err(e) => format!("Failed to run git: {}", e),
        }
    }

    pub fn collect(&self, path: &str) -> String {
        let tree = self.get_file_tree(path, 3);
        let status = self.get_git_status(path);

        format!(
            "## Workspace Context\n### File Tree (Depth 3)\n```\n{}\n```\n### Git Status\n```\n{}\n```",
            tree, status
        )
    }
}

impl Default for ContextEnricher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_file_tree() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        File::create(path.join("file1.txt")).unwrap();
        std::fs::create_dir(path.join("src")).unwrap();
        File::create(path.join("src/main.rs")).unwrap();

        let enricher = ContextEnricher::new();
        let tree = enricher.get_file_tree(path.to_str().unwrap(), 3);

        assert!(tree.contains("file1.txt"));
        assert!(tree.contains("src/"));
        assert!(tree.contains("main.rs"));
    }
}
