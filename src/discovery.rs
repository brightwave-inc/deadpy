use std::path::PathBuf;

use anyhow::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::config::Config;

pub fn discover_files(config: &Config) -> Result<Vec<PathBuf>> {
    let exclude_set = build_exclude_set(&config.exclude)?;
    let mut files = Vec::new();

    for path in &config.paths {
        let walker = WalkBuilder::new(path).hidden(true).git_ignore(true).build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("py") {
                continue;
            }

            let path_str = path.to_string_lossy();
            if exclude_set.is_match(path_str.as_ref()) {
                continue;
            }

            files.push(path.to_path_buf());
        }
    }

    files.sort();
    Ok(files)
}

fn build_exclude_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        // Support both path-style ("dir/") and glob-style ("*scratch*") patterns.
        // For patterns that are directory prefixes (end with /), add a glob suffix.
        let glob_pattern = if pattern.ends_with('/') {
            format!("**/{pattern}**")
        } else if !pattern.contains('*') && !pattern.contains('?') {
            // Plain directory/file name — match it anywhere in the path
            format!("**/{pattern}/**")
        } else {
            // Already a glob pattern — wrap to match anywhere
            format!("**/{pattern}")
        };
        builder.add(Glob::new(&glob_pattern)?);
    }
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_python_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("foo.py"), "x = 1").unwrap();
        fs::write(dir.path().join("bar.txt"), "text").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/baz.py"), "y = 2").unwrap();

        let config = Config {
            paths: vec![dir.path().to_path_buf()],
            ..Config::default()
        };

        let files = discover_files(&config).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.extension().unwrap() == "py"));
    }

    #[test]
    fn test_exclude_patterns() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("good.py"), "x = 1").unwrap();
        fs::create_dir_all(dir.path().join(".venv")).unwrap();
        fs::write(dir.path().join(".venv/bad.py"), "y = 2").unwrap();
        fs::create_dir_all(dir.path().join("__pycache__")).unwrap();
        fs::write(dir.path().join("__pycache__/cached.py"), "z = 3").unwrap();

        let config = Config {
            paths: vec![dir.path().to_path_buf()],
            exclude: vec![".venv/".to_string(), "__pycache__".to_string()],
            ..Config::default()
        };

        let files = discover_files(&config).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("good.py"));
    }
}
