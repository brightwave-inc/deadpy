use std::fmt;
use std::str::FromStr;

use crate::analysis::Finding;

#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Count,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "count" => Ok(OutputFormat::Count),
            other => Err(format!("unknown output format: {other}")),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Count => write!(f, "count"),
        }
    }
}

pub fn format_findings(findings: &[Finding], format: OutputFormat, sort_by_size: bool) -> String {
    let mut sorted: Vec<&Finding> = findings.iter().collect();

    if sort_by_size {
        sorted.sort_by_key(|f| std::cmp::Reverse(f.definition.size));
    } else {
        sorted.sort_by(|a, b| {
            a.definition
                .file
                .cmp(&b.definition.file)
                .then(a.definition.line.cmp(&b.definition.line))
        });
    }

    match format {
        OutputFormat::Text => format_text(&sorted),
        OutputFormat::Json => format_json(&sorted),
        OutputFormat::Count => format!("{}\n", sorted.len()),
    }
}

fn format_text(findings: &[&Finding]) -> String {
    let mut output = String::new();
    for f in findings {
        let def = &f.definition;
        output.push_str(&format!(
            "{}:{}: unused {} '{}' ({}% confidence)\n",
            def.file.display(),
            def.line,
            def.kind.as_str(),
            def.name,
            def.confidence,
        ));
    }
    output
}

fn format_json(findings: &[&Finding]) -> String {
    let entries: Vec<String> = findings
        .iter()
        .map(|f| {
            let def = &f.definition;
            format!(
                r#"  {{"file": "{}", "line": {}, "kind": "{}", "name": "{}", "confidence": {}, "size": {}}}"#,
                def.file.display(),
                def.line,
                def.kind.as_str(),
                def.name,
                def.confidence,
                def.size,
            )
        })
        .collect();
    format!("[\n{}\n]\n", entries.join(",\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Definition, DefinitionKind};
    use std::path::PathBuf;

    fn make_finding(name: &str, line: usize, size: usize) -> Finding {
        Finding {
            definition: Definition {
                name: name.to_string(),
                qualified_name: name.to_string(),
                kind: DefinitionKind::Function,
                file: PathBuf::from("test.py"),
                line,
                size,
                decorators: Vec::new(),
                confidence: 60,
            },
        }
    }

    #[test]
    fn test_text_format() {
        let findings = vec![make_finding("foo", 10, 5)];
        let output = format_findings(&findings, OutputFormat::Text, false);
        assert_eq!(
            output,
            "test.py:10: unused function 'foo' (60% confidence)\n"
        );
    }

    #[test]
    fn test_count_format() {
        let findings = vec![make_finding("a", 1, 1), make_finding("b", 2, 1)];
        let output = format_findings(&findings, OutputFormat::Count, false);
        assert_eq!(output, "2\n");
    }

    #[test]
    fn test_sort_by_size() {
        let findings = vec![make_finding("small", 1, 3), make_finding("big", 2, 30)];
        let output = format_findings(&findings, OutputFormat::Text, true);
        assert!(output.starts_with("test.py:2: unused function 'big'"));
    }

    #[test]
    fn test_json_format() {
        let findings = vec![make_finding("foo", 10, 5)];
        let output = format_findings(&findings, OutputFormat::Json, false);
        assert!(output.contains("\"name\": \"foo\""));
        assert!(output.contains("\"line\": 10"));
    }
}
