use std::path::PathBuf;
use std::process;

use anyhow::Result;
use clap::Parser;

use deadpy::analysis::analyze;
use deadpy::config::Config;
use deadpy::discovery::discover_files;
use deadpy::output::{format_findings, OutputFormat};
use deadpy::parser::parse_files;
use deadpy::whitelist::parse_whitelist_files;

#[derive(Parser)]
#[command(name = "deadpy", version, about = "Fast Python dead code detector")]
struct Cli {
    /// Paths to scan (overrides config)
    #[arg()]
    paths: Vec<PathBuf>,

    /// Path to pyproject.toml
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Additional exclude patterns
    #[arg(long, value_name = "PATTERN")]
    exclude: Vec<String>,

    /// Additional decorator patterns to ignore
    #[arg(long, value_name = "PATTERN")]
    ignore_decorators: Vec<String>,

    /// Additional name patterns to ignore
    #[arg(long, value_name = "PATTERN")]
    ignore_names: Vec<String>,

    /// Minimum confidence (0-100)
    #[arg(long, value_name = "N")]
    min_confidence: Option<u8>,

    /// Additional whitelist files
    #[arg(long, value_name = "FILE")]
    whitelist: Vec<PathBuf>,

    /// What to report: function,class,method,variable,import
    #[arg(long, value_name = "KINDS", value_delimiter = ',')]
    report: Vec<String>,

    /// Sort by code size instead of file:line
    #[arg(long)]
    sort_by_size: bool,

    /// Output format: text, json, count
    #[arg(long, value_name = "FMT", default_value = "text")]
    format: OutputFormat,

    /// Only output dead code findings (no summary)
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli
        .config
        .clone()
        .unwrap_or_else(|| PathBuf::from("pyproject.toml"));
    let mut config = Config::load(&config_path)?;

    // CLI overrides
    if !cli.paths.is_empty() {
        config.paths = cli.paths;
    }
    config.exclude.extend(cli.exclude);
    config.ignore_decorators.extend(cli.ignore_decorators);
    config.ignore_names.extend(cli.ignore_names);
    if let Some(min_conf) = cli.min_confidence {
        config.min_confidence = min_conf;
    }
    config.whitelist.extend(cli.whitelist);
    if !cli.report.is_empty() {
        config.report = cli.report;
    }
    if cli.sort_by_size {
        config.sort_by_size = true;
    }

    let files = discover_files(&config)?;

    if !cli.quiet {
        eprintln!("Scanning {} Python files...", files.len());
    }

    let file_analyses = parse_files(&files);
    let whitelist_refs = parse_whitelist_files(&config.whitelist)?;
    let findings = analyze(&file_analyses, &whitelist_refs, &config);

    if findings.is_empty() {
        if !cli.quiet {
            eprintln!("No dead code found.");
        }
        return Ok(());
    }

    let output = format_findings(&findings, cli.format, config.sort_by_size);
    print!("{output}");

    process::exit(1);
}
