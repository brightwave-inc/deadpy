use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::parser::parse_file;

/// Parse whitelist files using tree-sitter (just like any other Python file).
/// All identifiers found via imports and attribute accesses are treated as references.
pub fn parse_whitelist_files(paths: &[PathBuf]) -> Result<HashSet<String>> {
    let mut refs = HashSet::new();

    for path in paths {
        if !path.exists() {
            eprintln!(
                "Warning: whitelist file {} does not exist, skipping",
                path.display()
            );
            continue;
        }
        let file_refs = parse_whitelist_file(path)?;
        refs.extend(file_refs);
    }

    Ok(refs)
}

fn parse_whitelist_file(path: &Path) -> Result<HashSet<String>> {
    let analysis = parse_file(path)?;
    // Whitelist files' references (from imports and attribute accesses) act as global references
    // that prevent flagging. Also include names from definitions (e.g., bare attribute accesses
    // like `APIClient.async_get` which tree-sitter parses as an expression statement).
    let mut refs = analysis.references;

    // Also add all definition names from the whitelist — these are names that appear
    // as expression statements (like `APIClient.async_get`) parsed as variable definitions
    for def in &analysis.definitions {
        refs.insert(def.name.clone());
    }

    Ok(refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_whitelist_with_imports() {
        let mut f = NamedTempFile::with_suffix(".py").unwrap();
        writeln!(f, "from myapp.utils.formatting import pretty_print").unwrap();
        writeln!(f, "from myapp.models import Account").unwrap();
        f.flush().unwrap();

        let refs = parse_whitelist_files(&[f.path().to_path_buf()]).unwrap();
        assert!(refs.contains("pretty_print"));
        assert!(refs.contains("Account"));
    }

    #[test]
    fn test_parse_whitelist_with_attribute_access() {
        let mut f = NamedTempFile::with_suffix(".py").unwrap();
        writeln!(f, "from myapp.client import APIClient").unwrap();
        writeln!(f, "APIClient.async_get").unwrap();
        f.flush().unwrap();

        let refs = parse_whitelist_files(&[f.path().to_path_buf()]).unwrap();
        assert!(refs.contains("APIClient"));
        assert!(refs.contains("async_get"));
    }

    #[test]
    fn test_missing_whitelist_file() {
        let refs = parse_whitelist_files(&[PathBuf::from("nonexistent_whitelist.py")]).unwrap();
        assert!(refs.is_empty());
    }
}
