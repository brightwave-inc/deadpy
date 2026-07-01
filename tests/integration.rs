use std::collections::HashSet;
use std::path::PathBuf;

use deadpy::analysis::analyze;
use deadpy::config::Config;
use deadpy::parser::{parse_file, parse_files, DefinitionKind};
use deadpy::whitelist::parse_whitelist_files;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_simple_dead_alive() {
    let analysis = parse_file(&fixture_path("simple.py")).unwrap();

    let dead_names: Vec<&str> = vec!["unused_function", "UnusedClass"];
    let alive_names: Vec<&str> = vec!["used_function", "UsedClass"];

    let config = Config {
        report: vec![
            "function".to_string(),
            "class".to_string(),
            "method".to_string(),
        ],
        ..Config::default()
    };
    let findings = analyze(&[analysis], &HashSet::new(), &config);
    let flagged_names: Vec<&str> = findings
        .iter()
        .map(|f| f.definition.name.as_str())
        .collect();

    for name in &dead_names {
        assert!(
            flagged_names.contains(name),
            "{name} should be flagged as dead code"
        );
    }
    for name in &alive_names {
        assert!(
            !flagged_names.contains(name),
            "{name} should NOT be flagged (it's used)"
        );
    }
}

#[test]
fn test_all_exports_not_treated_as_references() {
    let analysis = parse_file(&fixture_path("all_exports.py")).unwrap();

    // Verify __all__ entries are captured
    assert!(analysis
        .all_exports
        .contains(&"exported_but_unused".to_string()));
    assert!(analysis.all_exports.contains(&"actually_used".to_string()));

    // Verify __all__ entries are NOT in references
    assert!(!analysis.references.contains("exported_but_unused"));

    let config = Config::default();
    let findings = analyze(&[analysis], &HashSet::new(), &config);
    let flagged: Vec<&str> = findings
        .iter()
        .map(|f| f.definition.name.as_str())
        .collect();

    // exported_but_unused: in __all__ but never called → should be flagged
    assert!(
        flagged.contains(&"exported_but_unused"),
        "exported_but_unused should be flagged (only in __all__, not actually used)"
    );

    // actually_used: in __all__ AND called → should NOT be flagged
    assert!(
        !flagged.contains(&"actually_used"),
        "actually_used should NOT be flagged (it's actually called)"
    );

    // internal_helper: not in __all__, not called → should be flagged
    assert!(
        flagged.contains(&"internal_helper"),
        "internal_helper should be flagged"
    );
}

#[test]
fn test_decorator_ignore() {
    let analysis = parse_file(&fixture_path("decorators.py")).unwrap();

    let config = Config {
        ignore_decorators: vec!["@router.get".to_string(), "@router.post".to_string()],
        ..Config::default()
    };
    let findings = analyze(&[analysis], &HashSet::new(), &config);
    let flagged: Vec<&str> = findings
        .iter()
        .map(|f| f.definition.name.as_str())
        .collect();

    assert!(
        !flagged.contains(&"list_items"),
        "list_items has @router.get — should be ignored"
    );
    assert!(
        !flagged.contains(&"create_item"),
        "create_item has @router.post — should be ignored"
    );
    assert!(
        flagged.contains(&"undecorated_unused"),
        "undecorated_unused should be flagged"
    );
}

#[test]
fn test_dunder_auto_whitelist() {
    let analysis = parse_file(&fixture_path("dunders.py")).unwrap();

    let config = Config {
        report: vec!["method".to_string()],
        ..Config::default()
    };
    let findings = analyze(&[analysis], &HashSet::new(), &config);
    let flagged: Vec<&str> = findings
        .iter()
        .map(|f| f.definition.name.as_str())
        .collect();

    assert!(
        !flagged.contains(&"__init__"),
        "__init__ should be auto-whitelisted"
    );
    assert!(
        !flagged.contains(&"__str__"),
        "__str__ should be auto-whitelisted"
    );
    assert!(
        !flagged.contains(&"__repr__"),
        "__repr__ should be auto-whitelisted"
    );
    assert!(
        !flagged.contains(&"__enter__"),
        "__enter__ should be auto-whitelisted"
    );
    assert!(
        !flagged.contains(&"__exit__"),
        "__exit__ should be auto-whitelisted"
    );
    assert!(
        flagged.contains(&"unused_method"),
        "unused_method should be flagged"
    );
}

#[test]
fn test_whitelist_file_parsing() {
    let wl_refs = parse_whitelist_files(&[fixture_path("whitelist.py")]).unwrap();

    assert!(
        wl_refs.contains("whitelisted_function"),
        "whitelist should add whitelisted_function as a reference"
    );
    assert!(
        wl_refs.contains("whitelisted_method"),
        "whitelist should add whitelisted_method as a reference"
    );
    assert!(
        wl_refs.contains("WhitelistedClass"),
        "whitelist should add WhitelistedClass as a reference"
    );
}

#[test]
fn test_whitelist_prevents_flagging() {
    let analysis = parse_file(&fixture_path("whitelist_target.py")).unwrap();
    let wl_refs = parse_whitelist_files(&[fixture_path("whitelist.py")]).unwrap();

    let config = Config {
        report: vec![
            "function".to_string(),
            "class".to_string(),
            "method".to_string(),
        ],
        ..Config::default()
    };
    let findings = analyze(&[analysis], &wl_refs, &config);
    let flagged: Vec<&str> = findings
        .iter()
        .map(|f| f.definition.name.as_str())
        .collect();

    assert!(
        !flagged.contains(&"whitelisted_function"),
        "whitelisted_function should be saved by whitelist"
    );
    assert!(
        !flagged.contains(&"whitelisted_method"),
        "whitelisted_method should be saved by whitelist"
    );
    assert!(
        flagged.contains(&"not_whitelisted"),
        "not_whitelisted should be flagged"
    );
}

#[test]
fn test_parallel_parsing() {
    let files = vec![
        fixture_path("simple.py"),
        fixture_path("all_exports.py"),
        fixture_path("decorators.py"),
        fixture_path("dunders.py"),
    ];

    let analyses = parse_files(&files);
    assert_eq!(analyses.len(), 4);

    // Each file should have at least one definition
    for analysis in &analyses {
        assert!(
            !analysis.definitions.is_empty(),
            "File {} should have definitions",
            analysis.path.display()
        );
    }
}

#[test]
fn test_name_ignore_with_wildcards() {
    let analysis = parse_file(&fixture_path("simple.py")).unwrap();

    let config = Config {
        ignore_names: vec!["unused_*".to_string()],
        ..Config::default()
    };
    let findings = analyze(&[analysis], &HashSet::new(), &config);
    let flagged: Vec<&str> = findings
        .iter()
        .map(|f| f.definition.name.as_str())
        .collect();

    assert!(
        !flagged.contains(&"unused_function"),
        "unused_function should match 'unused_*' pattern"
    );
}

#[test]
fn test_method_qualified_name() {
    let analysis = parse_file(&fixture_path("dunders.py")).unwrap();
    let methods: Vec<_> = analysis
        .definitions
        .iter()
        .filter(|d| d.kind == DefinitionKind::Method)
        .collect();

    for method in &methods {
        assert!(
            method.qualified_name.starts_with("MyClass."),
            "Method {} should have qualified name starting with 'MyClass.'",
            method.name
        );
    }
}
