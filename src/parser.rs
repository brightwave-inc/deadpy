use std::collections::HashSet;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

/// Set of dunder methods automatically whitelisted (they're part of the Python data model).
const WHITELISTED_DUNDERS: &[&str] = &[
    "__init__",
    "__new__",
    "__del__",
    "__repr__",
    "__str__",
    "__bytes__",
    "__format__",
    "__lt__",
    "__le__",
    "__eq__",
    "__ne__",
    "__gt__",
    "__ge__",
    "__hash__",
    "__bool__",
    "__getattr__",
    "__getattribute__",
    "__setattr__",
    "__delattr__",
    "__dir__",
    "__get__",
    "__set__",
    "__delete__",
    "__set_name__",
    "__init_subclass__",
    "__class_getitem__",
    "__call__",
    "__len__",
    "__length_hint__",
    "__getitem__",
    "__setitem__",
    "__delitem__",
    "__missing__",
    "__iter__",
    "__next__",
    "__reversed__",
    "__contains__",
    "__add__",
    "__radd__",
    "__iadd__",
    "__mul__",
    "__rmul__",
    "__imul__",
    "__sub__",
    "__rsub__",
    "__isub__",
    "__mod__",
    "__rmod__",
    "__imod__",
    "__pow__",
    "__rpow__",
    "__ipow__",
    "__truediv__",
    "__rtruediv__",
    "__itruediv__",
    "__floordiv__",
    "__rfloordiv__",
    "__ifloordiv__",
    "__neg__",
    "__pos__",
    "__abs__",
    "__invert__",
    "__lshift__",
    "__rlshift__",
    "__ilshift__",
    "__rshift__",
    "__rrshift__",
    "__irshift__",
    "__and__",
    "__rand__",
    "__iand__",
    "__or__",
    "__ror__",
    "__ior__",
    "__xor__",
    "__rxor__",
    "__ixor__",
    "__int__",
    "__float__",
    "__complex__",
    "__index__",
    "__round__",
    "__trunc__",
    "__floor__",
    "__ceil__",
    "__enter__",
    "__exit__",
    "__aenter__",
    "__aexit__",
    "__aiter__",
    "__anext__",
    "__await__",
    "__fspath__",
    "__reduce__",
    "__reduce_ex__",
    "__getstate__",
    "__setstate__",
    "__copy__",
    "__deepcopy__",
    "__getnewargs__",
    "__getnewargs_ex__",
    "__sizeof__",
    "__subclasshook__",
    "__instancecheck__",
    "__subclasscheck__",
    "__class__",
    "__dict__",
    "__doc__",
    "__module__",
    "__slots__",
    "__weakref__",
    "__abstractmethods__",
    "__all__",
    "__annotations__",
    "__bases__",
    "__builtins__",
    "__cached__",
    "__file__",
    "__loader__",
    "__name__",
    "__package__",
    "__path__",
    "__qualname__",
    "__spec__",
    "__wrapped__",
    "__traceback__",
    "__cause__",
    "__context__",
    "__suppress_context__",
    "__args__",
    "__origin__",
    "__mro__",
    "__mro_entries__",
    "__prepare__",
    "__post_init__",
    "__match_args__",
    "__type_params__",
    "__buffer__",
    "__release_buffer__",
    "__get_pydantic_core_schema__",
    "__get_validators__",
    "__modify_schema__",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionKind {
    Function,
    Class,
    Method,
    Variable,
    Import,
}

impl DefinitionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DefinitionKind::Function => "function",
            DefinitionKind::Class => "class",
            DefinitionKind::Method => "method",
            DefinitionKind::Variable => "variable",
            DefinitionKind::Import => "import",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub name: String,
    pub qualified_name: String,
    pub kind: DefinitionKind,
    pub file: PathBuf,
    pub line: usize,
    pub size: usize,
    pub decorators: Vec<String>,
    pub confidence: u8,
}

#[derive(Debug)]
pub struct FileAnalysis {
    pub path: PathBuf,
    pub definitions: Vec<Definition>,
    pub references: HashSet<String>,
    pub all_exports: Vec<String>,
}

/// Parse multiple Python files in parallel.
pub fn parse_files(files: &[PathBuf]) -> Vec<FileAnalysis> {
    files
        .par_iter()
        .filter_map(|path| match parse_file(path) {
            Ok(analysis) => Some(analysis),
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {e}", path.display());
                None
            }
        })
        .collect()
}

/// Parse a single Python file and extract definitions, references, and __all__ entries.
pub fn parse_file(path: &Path) -> anyhow::Result<FileAnalysis> {
    let source = std::fs::read_to_string(path)?;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into())?;

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter parse failed for {}", path.display()))?;

    let source_bytes = source.as_bytes();
    let root = tree.root_node();

    let mut definitions = Vec::new();
    let mut references = HashSet::new();
    let mut all_exports = Vec::new();

    extract_from_node(
        root,
        source_bytes,
        path,
        None, // no enclosing class
        &mut definitions,
        &mut references,
        &mut all_exports,
    );

    Ok(FileAnalysis {
        path: path.to_path_buf(),
        definitions,
        references,
        all_exports,
    })
}

fn extract_from_node(
    node: tree_sitter::Node,
    source: &[u8],
    path: &Path,
    enclosing_class: Option<&str>,
    definitions: &mut Vec<Definition>,
    references: &mut HashSet<String>,
    all_exports: &mut Vec<String>,
) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "decorated_definition" => {
                handle_decorated_definition(
                    child,
                    source,
                    path,
                    enclosing_class,
                    definitions,
                    references,
                    all_exports,
                );
            }
            "function_definition" => {
                handle_function_definition(
                    child,
                    source,
                    path,
                    enclosing_class,
                    &[],
                    definitions,
                    references,
                    all_exports,
                );
            }
            "class_definition" => {
                handle_class_definition(
                    child,
                    source,
                    path,
                    &[],
                    definitions,
                    references,
                    all_exports,
                );
            }
            "expression_statement" => {
                // Check for __all__ assignment
                handle_expression_statement(
                    child,
                    source,
                    path,
                    definitions,
                    references,
                    all_exports,
                );
            }
            "import_from_statement" => {
                handle_import_from(child, source, references);
            }
            "import_statement" => {
                handle_import(child, source, references);
            }
            "identifier" => {
                let name = node_text(child, source);
                references.insert(name);
            }
            "attribute" => {
                extract_attribute_references(child, source, references);
            }
            "call" => {
                // Extract the function being called as a reference, then recurse into arguments
                extract_call_references(child, source, references);
                extract_from_node(
                    child,
                    source,
                    path,
                    enclosing_class,
                    definitions,
                    references,
                    all_exports,
                );
            }
            "string" | "concatenated_string" => {
                // Don't extract identifiers from plain string content, but DO
                // recurse into f-string interpolations (e.g. `f"{some_fn()}"`)
                extract_fstring_references(child, source, references);
            }
            "comment" => {}
            _ => {
                extract_from_node(
                    child,
                    source,
                    path,
                    enclosing_class,
                    definitions,
                    references,
                    all_exports,
                );
            }
        }
    }
}

fn handle_decorated_definition(
    node: tree_sitter::Node,
    source: &[u8],
    path: &Path,
    enclosing_class: Option<&str>,
    definitions: &mut Vec<Definition>,
    references: &mut HashSet<String>,
    all_exports: &mut Vec<String>,
) {
    let mut decorators = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "decorator" => {
                let dec_text = node_text(child, source);
                decorators.push(dec_text);
                // Also add decorator identifiers as references
                extract_decorator_references(child, source, references);
            }
            "function_definition" => {
                handle_function_definition(
                    child,
                    source,
                    path,
                    enclosing_class,
                    &decorators,
                    definitions,
                    references,
                    all_exports,
                );
            }
            "class_definition" => {
                handle_class_definition(
                    child,
                    source,
                    path,
                    &decorators,
                    definitions,
                    references,
                    all_exports,
                );
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_function_definition(
    node: tree_sitter::Node,
    source: &[u8],
    path: &Path,
    enclosing_class: Option<&str>,
    decorators: &[String],
    definitions: &mut Vec<Definition>,
    references: &mut HashSet<String>,
    all_exports: &mut Vec<String>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))
        .unwrap_or_default();

    if name.is_empty() {
        return;
    }

    let kind = if enclosing_class.is_some() {
        DefinitionKind::Method
    } else {
        DefinitionKind::Function
    };

    let qualified_name = match enclosing_class {
        Some(cls) => format!("{cls}.{name}"),
        None => name.clone(),
    };

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let size = end_line.saturating_sub(start_line) + 1;

    let confidence = match kind {
        DefinitionKind::Method => 60,
        DefinitionKind::Function => 60,
        _ => 60,
    };

    definitions.push(Definition {
        name,
        qualified_name,
        kind,
        file: path.to_path_buf(),
        line: start_line,
        size,
        decorators: decorators.to_vec(),
        confidence,
    });

    // Recurse into function body for references (but not as defining scope for new definitions)
    let body = node.child_by_field_name("body");
    if let Some(body) = body {
        extract_from_node(
            body,
            source,
            path,
            enclosing_class,
            &mut Vec::new(), // don't collect nested function defs as top-level
            references,
            all_exports,
        );
    }

    // Extract references from parameters (type annotations, defaults)
    if let Some(params) = node.child_by_field_name("parameters") {
        extract_references_from_subtree(params, source, references);
    }
    // Extract references from return type annotation
    if let Some(ret) = node.child_by_field_name("return_type") {
        extract_references_from_subtree(ret, source, references);
    }
}

fn handle_class_definition(
    node: tree_sitter::Node,
    source: &[u8],
    path: &Path,
    decorators: &[String],
    definitions: &mut Vec<Definition>,
    references: &mut HashSet<String>,
    all_exports: &mut Vec<String>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))
        .unwrap_or_default();

    if name.is_empty() {
        return;
    }

    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    let size = end_line.saturating_sub(start_line) + 1;

    definitions.push(Definition {
        name: name.clone(),
        qualified_name: name.clone(),
        kind: DefinitionKind::Class,
        file: path.to_path_buf(),
        line: start_line,
        size,
        decorators: decorators.to_vec(),
        confidence: 60,
    });

    // Extract base class references
    if let Some(superclasses) = node.child_by_field_name("superclasses") {
        extract_references_from_subtree(superclasses, source, references);
    }

    // Recurse into class body with this class as enclosing class
    let body = node.child_by_field_name("body");
    if let Some(body) = body {
        extract_from_node(
            body,
            source,
            path,
            Some(&name),
            definitions,
            references,
            all_exports,
        );
    }
}

fn handle_expression_statement(
    node: tree_sitter::Node,
    source: &[u8],
    path: &Path,
    definitions: &mut Vec<Definition>,
    references: &mut HashSet<String>,
    all_exports: &mut Vec<String>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "assignment" {
            handle_assignment(child, source, path, definitions, references, all_exports);
        } else if child.kind() == "augmented_assignment" {
            // e.g., __all__ += [...] — skip for now, just extract references
            extract_references_from_subtree(child, source, references);
        } else {
            // For other expressions, extract references normally
            extract_references_from_subtree(child, source, references);
        }
    }
}

fn handle_assignment(
    node: tree_sitter::Node,
    source: &[u8],
    path: &Path,
    definitions: &mut Vec<Definition>,
    references: &mut HashSet<String>,
    all_exports: &mut Vec<String>,
) {
    let left = node.child_by_field_name("left");
    let right = node.child_by_field_name("right");

    let target_name = left.map(|n| node_text(n, source)).unwrap_or_default();

    if target_name == "__all__" {
        // Parse __all__ entries but do NOT add them to references
        if let Some(right) = right {
            extract_all_entries(right, source, all_exports);
        }
        return;
    }

    // For other module-level assignments, record as variable definitions
    if !target_name.is_empty()
        && target_name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        && left.is_some_and(|n| n.kind() == "identifier")
    {
        let start_line = node.start_position().row + 1;
        definitions.push(Definition {
            name: target_name,
            qualified_name: String::new(), // filled in later if needed
            kind: DefinitionKind::Variable,
            file: path.to_path_buf(),
            line: start_line,
            size: 1,
            decorators: Vec::new(),
            confidence: 60,
        });
    }

    // Extract references from the right-hand side
    if let Some(right) = right {
        extract_references_from_subtree(right, source, references);
    }
    // Also extract references from type annotation if present
    if let Some(ann) = node.child_by_field_name("type") {
        extract_references_from_subtree(ann, source, references);
    }
}

fn extract_all_entries(node: tree_sitter::Node, source: &[u8], all_exports: &mut Vec<String>) {
    match node.kind() {
        "list" | "tuple" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "string" {
                    if let Some(content) = extract_string_content(child, source) {
                        all_exports.push(content);
                    }
                }
            }
        }
        _ => {
            // __all__ is set to something we can't parse statically
        }
    }
}

fn extract_string_content(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // String nodes contain the quotes; extract the content within.
    // tree-sitter-python has `string_content` child nodes.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string_content" {
            return Some(node_text(child, source));
        }
    }
    // Fallback: strip quotes manually
    let text = node_text(node, source);
    let trimmed = text
        .trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_start_matches("'''")
        .trim_end_matches("'''")
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_start_matches('\'')
        .trim_end_matches('\'');
    if trimmed != text {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn handle_import_from(node: tree_sitter::Node, source: &[u8], references: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "dotted_name" => {
                // Module path — add each part as a reference
                let mut name_cursor = child.walk();
                for part in child.children(&mut name_cursor) {
                    if part.kind() == "identifier" {
                        references.insert(node_text(part, source));
                    }
                }
            }
            "aliased_import" => {
                // `from x import y as z` — y is the reference
                if let Some(name_node) = child.child_by_field_name("name") {
                    references.insert(node_text(name_node, source));
                }
            }
            "identifier" => {
                // Imported name
                references.insert(node_text(child, source));
            }
            _ => {}
        }
    }
}

fn handle_import(node: tree_sitter::Node, source: &[u8], references: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
            extract_references_from_subtree(child, source, references);
        }
    }
}

fn extract_attribute_references(
    node: tree_sitter::Node,
    source: &[u8],
    references: &mut HashSet<String>,
) {
    // For `a.b.c`, we want references to "a", "b", and "c"
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                references.insert(node_text(child, source));
            }
            "attribute" => {
                extract_attribute_references(child, source, references);
            }
            _ => {
                // The object of an attribute access can be an arbitrary expression,
                // e.g. the `A()` in `A().m()` or the `items[0]` in `items[0].m`.
                // Recurse so nested references (like `A`) aren't lost.
                extract_references_from_subtree(child, source, references);
            }
        }
    }
}

fn extract_call_references(
    node: tree_sitter::Node,
    source: &[u8],
    references: &mut HashSet<String>,
) {
    if let Some(func) = node.child_by_field_name("function") {
        match func.kind() {
            "identifier" => {
                references.insert(node_text(func, source));
            }
            "attribute" => {
                extract_attribute_references(func, source, references);
            }
            _ => {
                // The callee can itself be an expression, e.g. `factory()()` or
                // `handlers[key]()`. Recurse so nested references aren't lost.
                extract_references_from_subtree(func, source, references);
            }
        }
    }
}

fn extract_decorator_references(
    node: tree_sitter::Node,
    source: &[u8],
    references: &mut HashSet<String>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                references.insert(node_text(child, source));
            }
            "attribute" => {
                extract_attribute_references(child, source, references);
            }
            "call" => {
                extract_call_references(child, source, references);
                // Also recurse into call arguments for references
                extract_references_from_subtree(child, source, references);
            }
            _ => {
                extract_decorator_references(child, source, references);
            }
        }
    }
}

fn extract_references_from_subtree(
    node: tree_sitter::Node,
    source: &[u8],
    references: &mut HashSet<String>,
) {
    match node.kind() {
        "identifier" => {
            references.insert(node_text(node, source));
        }
        "attribute" => {
            extract_attribute_references(node, source, references);
        }
        "string" | "concatenated_string" => {
            // Don't extract identifiers from plain string content, but DO
            // recurse into f-string interpolations (e.g. `f"{some_fn()}"`)
            extract_fstring_references(node, source, references);
        }
        "comment" => {}
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                extract_references_from_subtree(child, source, references);
            }
        }
    }
}

/// Recurse into f-string interpolation nodes to extract references.
/// Plain string content is skipped; only `interpolation` children are walked.
fn extract_fstring_references(
    node: tree_sitter::Node,
    source: &[u8],
    references: &mut HashSet<String>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "interpolation" {
            extract_references_from_subtree(child, source, references);
        } else if child.kind() == "concatenated_string" || child.kind() == "string" {
            extract_fstring_references(child, source, references);
        }
        // Skip string_start, string_content, string_end, escape_sequence, etc.
    }
}

fn node_text(node: tree_sitter::Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or("").to_string()
}

/// Returns true if the name is a dunder that should be auto-whitelisted.
pub fn is_whitelisted_dunder(name: &str) -> bool {
    WHITELISTED_DUNDERS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn parse_source(code: &str) -> FileAnalysis {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(code.as_bytes()).unwrap();
        f.flush().unwrap();
        parse_file(f.path()).unwrap()
    }

    #[test]
    fn test_function_definition() {
        let analysis = parse_source("def foo():\n    pass\n");
        assert_eq!(analysis.definitions.len(), 1);
        assert_eq!(analysis.definitions[0].name, "foo");
        assert_eq!(analysis.definitions[0].kind, DefinitionKind::Function);
    }

    #[test]
    fn test_class_definition() {
        let analysis = parse_source("class MyClass:\n    pass\n");
        assert_eq!(analysis.definitions.len(), 1);
        assert_eq!(analysis.definitions[0].name, "MyClass");
        assert_eq!(analysis.definitions[0].kind, DefinitionKind::Class);
    }

    #[test]
    fn test_method_definition() {
        let analysis = parse_source("class Foo:\n    def bar(self):\n        pass\n");
        let methods: Vec<_> = analysis
            .definitions
            .iter()
            .filter(|d| d.kind == DefinitionKind::Method)
            .collect();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name, "bar");
        assert_eq!(methods[0].qualified_name, "Foo.bar");
    }

    #[test]
    fn test_all_exports_not_in_references() {
        let analysis = parse_source(
            r#"
__all__ = ["foo", "bar"]

def foo():
    pass

def bar():
    pass
"#,
        );
        assert_eq!(analysis.all_exports, vec!["foo", "bar"]);
        assert!(!analysis.references.contains("foo"));
        assert!(!analysis.references.contains("bar"));
    }

    #[test]
    fn test_import_adds_reference() {
        let analysis = parse_source("from os.path import join\n\njoin('a', 'b')\n");
        assert!(analysis.references.contains("join"));
        assert!(analysis.references.contains("os"));
        assert!(analysis.references.contains("path"));
    }

    #[test]
    fn test_attribute_references() {
        let analysis = parse_source("import os\nos.path.join('a', 'b')\n");
        assert!(analysis.references.contains("os"));
        assert!(analysis.references.contains("path"));
        assert!(analysis.references.contains("join"));
    }

    #[test]
    fn test_decorated_function() {
        let analysis = parse_source("@router.get\ndef endpoint():\n    pass\n");
        assert_eq!(analysis.definitions.len(), 1);
        assert!(!analysis.definitions[0].decorators.is_empty());
        let dec = &analysis.definitions[0].decorators[0];
        assert!(dec.contains("router"));
    }

    #[test]
    fn test_fstring_interpolation_references() {
        // Function called only inside an f-string format spec should be seen as a reference
        let analysis = parse_source(
            r#"
def helper(x):
    return x

def main():
    value = 1.5
    result = f"{value:.{helper('fmt')}f}"
    return result
"#,
        );
        assert!(
            analysis.references.contains("helper"),
            "expected 'helper' in references, got: {:?}",
            analysis.references
        );
    }

    #[test]
    fn test_chained_call_receiver_references() {
        // The receiver of a chained call — `A` in `A().m()` — must be recorded as a
        // reference, otherwise `A` is falsely flagged as unused.
        let analysis = parse_source("A().m()\n");
        assert!(
            analysis.references.contains("A"),
            "expected 'A' in references, got: {:?}",
            analysis.references
        );

        // Same for a plain function whose result is used in a chain: `f` in `f().bar()`.
        let analysis = parse_source("f().bit_length()\n");
        assert!(
            analysis.references.contains("f"),
            "expected 'f' in references, got: {:?}",
            analysis.references
        );

        // And a subscript receiver: `items` in `items[0].m()`.
        let analysis = parse_source("items[0].m()\n");
        assert!(
            analysis.references.contains("items"),
            "expected 'items' in references, got: {:?}",
            analysis.references
        );
    }

    #[test]
    fn test_dunder_whitelist() {
        assert!(is_whitelisted_dunder("__init__"));
        assert!(is_whitelisted_dunder("__str__"));
        assert!(!is_whitelisted_dunder("__custom__"));
        assert!(!is_whitelisted_dunder("regular_name"));
    }
}
