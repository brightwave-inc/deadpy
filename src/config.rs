use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Config {
    pub paths: Vec<PathBuf>,
    pub exclude: Vec<String>,
    pub ignore_names: Vec<String>,
    pub ignore_decorators: Vec<String>,
    pub min_confidence: u8,
    pub sort_by_size: bool,
    pub report: Vec<String>,
    pub whitelist: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            exclude: Vec::new(),
            ignore_names: Vec::new(),
            ignore_decorators: Vec::new(),
            min_confidence: 60,
            sort_by_size: false,
            report: vec![
                "function".to_string(),
                "class".to_string(),
                "method".to_string(),
            ],
            whitelist: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PyProjectToml {
    tool: Option<ToolSection>,
}

#[derive(Debug, Deserialize)]
struct ToolSection {
    deadpy: Option<RawToolConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct RawToolConfig {
    paths: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    ignore_names: Option<Vec<String>>,
    ignore_decorators: Option<Vec<String>>,
    min_confidence: Option<u8>,
    sort_by_size: Option<bool>,
    report: Option<Vec<String>>,
    whitelist: Option<Vec<String>>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let defaults = Config::default();

        if !path.exists() {
            return Ok(defaults);
        }

        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let pyproject: PyProjectToml =
            toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;

        let raw = pyproject.tool.and_then(|t| t.deadpy).unwrap_or_default();

        Ok(Config {
            paths: raw
                .paths
                .map(|ps| ps.into_iter().map(PathBuf::from).collect())
                .unwrap_or(defaults.paths),
            exclude: raw.exclude.unwrap_or(defaults.exclude),
            ignore_names: raw.ignore_names.unwrap_or(defaults.ignore_names),
            ignore_decorators: raw.ignore_decorators.unwrap_or(defaults.ignore_decorators),
            min_confidence: raw.min_confidence.unwrap_or(defaults.min_confidence),
            sort_by_size: raw.sort_by_size.unwrap_or(defaults.sort_by_size),
            report: raw.report.unwrap_or(defaults.report),
            whitelist: raw
                .whitelist
                .map(|ws| ws.into_iter().map(PathBuf::from).collect())
                .unwrap_or(defaults.whitelist),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.min_confidence, 60);
        assert_eq!(config.report, vec!["function", "class", "method"]);
        assert!(!config.sort_by_size);
    }

    #[test]
    fn test_load_missing_file() {
        let config = Config::load(Path::new("nonexistent.toml")).unwrap();
        assert_eq!(config.min_confidence, 60);
    }

    #[test]
    fn test_load_deadpy_section() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
[tool.deadpy]
paths = ["src/"]
exclude = [".venv/"]
min_confidence = 80
sort_by_size = true
report = ["function"]
whitelist = ["wl.py"]
ignore_names = ["foo"]
ignore_decorators = ["@bar"]
"#
        )
        .unwrap();

        let config = Config::load(f.path()).unwrap();
        assert_eq!(config.paths, vec![PathBuf::from("src/")]);
        assert_eq!(config.exclude, vec![".venv/"]);
        assert_eq!(config.min_confidence, 80);
        assert!(config.sort_by_size);
        assert_eq!(config.report, vec!["function"]);
        assert_eq!(config.whitelist, vec![PathBuf::from("wl.py")]);
        assert_eq!(config.ignore_names, vec!["foo"]);
        assert_eq!(config.ignore_decorators, vec!["@bar"]);
    }
}
