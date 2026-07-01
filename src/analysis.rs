use std::collections::HashSet;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::config::Config;
use crate::parser::{is_whitelisted_dunder, Definition, FileAnalysis};

/// A dead code finding ready for output.
#[derive(Debug, Clone)]
pub struct Finding {
    pub definition: Definition,
}

/// Run dead code analysis across all parsed files.
pub fn analyze(
    file_analyses: &[FileAnalysis],
    whitelist_refs: &HashSet<String>,
    config: &Config,
) -> Vec<Finding> {
    // Build the global reference set from all files + whitelist
    let mut global_refs: HashSet<String> = whitelist_refs.clone();
    for analysis in file_analyses {
        for r in &analysis.references {
            global_refs.insert(r.clone());
        }
    }

    // Build ignore-name matcher
    let name_ignore = build_name_ignore_set(&config.ignore_names);

    // Normalize decorator ignore patterns
    let decorator_ignores: Vec<String> = config
        .ignore_decorators
        .iter()
        .map(|d| d.trim_start_matches('@').to_string())
        .collect();

    // Build report filter
    let report_kinds: HashSet<&str> = config.report.iter().map(|s| s.as_str()).collect();

    let mut findings = Vec::new();

    for analysis in file_analyses {
        for def in &analysis.definitions {
            // Only report requested definition kinds
            if !report_kinds.contains(def.kind.as_str()) {
                continue;
            }

            // Skip dunder methods
            if is_whitelisted_dunder(&def.name) {
                continue;
            }

            // Skip if confidence below threshold
            if def.confidence < config.min_confidence {
                continue;
            }

            // Skip if name matches an ignore pattern
            if matches_name_ignore(&def.name, &name_ignore) {
                continue;
            }

            // Skip if any decorator matches an ignore pattern
            if has_ignored_decorator(&def.decorators, &decorator_ignores) {
                continue;
            }

            // The core check: is this name referenced anywhere?
            if global_refs.contains(&def.name) {
                continue;
            }

            // Also check the qualified name (for methods like Class.method)
            if !def.qualified_name.is_empty() && global_refs.contains(&def.qualified_name) {
                continue;
            }

            findings.push(Finding {
                definition: def.clone(),
            });
        }
    }

    findings
}

fn build_name_ignore_set(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().ok()
}

fn matches_name_ignore(name: &str, ignore_set: &Option<GlobSet>) -> bool {
    match ignore_set {
        Some(set) => set.is_match(name),
        None => false,
    }
}

fn has_ignored_decorator(decorators: &[String], ignore_patterns: &[String]) -> bool {
    for dec in decorators {
        // Strip the leading @ from the decorator text
        let dec_text = dec.trim_start_matches('@');
        for pattern in ignore_patterns {
            if decorator_matches(dec_text, pattern) {
                return true;
            }
        }
    }
    false
}

/// Check if a decorator text matches a pattern.
/// Pattern `router.get` matches decorators like `@router.get`, `@router.get("/path")`, etc.
fn decorator_matches(dec_text: &str, pattern: &str) -> bool {
    // The decorator text may include arguments like `router.get("/path")`
    // Check if it starts with or equals the pattern
    let base = dec_text.split('(').next().unwrap_or(dec_text).trim();
    base == pattern
        || base.starts_with(&format!("{pattern}."))
        || base.starts_with(&format!("{pattern}("))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::DefinitionKind;
    use std::path::PathBuf;

    fn make_def(name: &str, kind: DefinitionKind) -> Definition {
        Definition {
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind,
            file: PathBuf::from("test.py"),
            line: 1,
            size: 1,
            decorators: Vec::new(),
            confidence: 60,
        }
    }

    fn make_analysis(defs: Vec<Definition>, refs: HashSet<String>) -> FileAnalysis {
        FileAnalysis {
            path: PathBuf::from("test.py"),
            definitions: defs,
            references: refs,
            all_exports: Vec::new(),
        }
    }

    #[test]
    fn test_unused_function_flagged() {
        let analysis = make_analysis(
            vec![make_def("unused_fn", DefinitionKind::Function)],
            HashSet::new(),
        );
        let config = Config::default();
        let findings = analyze(&[analysis], &HashSet::new(), &config);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].definition.name, "unused_fn");
    }

    #[test]
    fn test_used_function_not_flagged() {
        let mut refs = HashSet::new();
        refs.insert("used_fn".to_string());
        let analysis = make_analysis(vec![make_def("used_fn", DefinitionKind::Function)], refs);
        let config = Config::default();
        let findings = analyze(&[analysis], &HashSet::new(), &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_cross_file_reference() {
        let analysis1 = make_analysis(
            vec![make_def("shared_fn", DefinitionKind::Function)],
            HashSet::new(),
        );
        let mut refs = HashSet::new();
        refs.insert("shared_fn".to_string());
        let analysis2 = make_analysis(Vec::new(), refs);

        let config = Config::default();
        let findings = analyze(&[analysis1, analysis2], &HashSet::new(), &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_whitelist_prevents_flagging() {
        let analysis = make_analysis(
            vec![make_def("whitelisted", DefinitionKind::Function)],
            HashSet::new(),
        );
        let mut wl = HashSet::new();
        wl.insert("whitelisted".to_string());
        let config = Config::default();
        let findings = analyze(&[analysis], &wl, &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ignore_names() {
        let analysis = make_analysis(
            vec![make_def("ITEM_FOO", DefinitionKind::Variable)],
            HashSet::new(),
        );
        let config = Config {
            ignore_names: vec!["ITEM_*".to_string()],
            report: vec!["variable".to_string()],
            ..Config::default()
        };
        let findings = analyze(&[analysis], &HashSet::new(), &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ignore_decorators() {
        let mut def = make_def("my_endpoint", DefinitionKind::Function);
        def.decorators = vec!["@router.get".to_string()];
        let analysis = make_analysis(vec![def], HashSet::new());
        let config = Config {
            ignore_decorators: vec!["@router.get".to_string()],
            ..Config::default()
        };
        let findings = analyze(&[analysis], &HashSet::new(), &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_dunder_auto_whitelisted() {
        let analysis = make_analysis(
            vec![make_def("__init__", DefinitionKind::Method)],
            HashSet::new(),
        );
        let config = Config {
            report: vec!["method".to_string()],
            ..Config::default()
        };
        let findings = analyze(&[analysis], &HashSet::new(), &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_report_filter() {
        let analysis = make_analysis(
            vec![
                make_def("unused_fn", DefinitionKind::Function),
                make_def("UnusedClass", DefinitionKind::Class),
            ],
            HashSet::new(),
        );
        let config = Config {
            report: vec!["function".to_string()], // only report functions
            ..Config::default()
        };
        let findings = analyze(&[analysis], &HashSet::new(), &config);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].definition.name, "unused_fn");
    }

    #[test]
    fn test_decorator_matching() {
        assert!(decorator_matches("router.get", "router.get"));
        assert!(decorator_matches("router.get(\"/path\")", "router.get"));
        assert!(!decorator_matches("router.post", "router.get"));
        assert!(decorator_matches("app.command", "app.command"));
    }
}
