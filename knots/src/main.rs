use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Tree, TreeCursor};
use walkdir::WalkDir;

mod complexity;
use complexity::{
    calculate_abc_complexity, calculate_cognitive_complexity, calculate_mccabe_complexity,
    calculate_nesting_depth, calculate_return_count, calculate_sloc, calculate_test_scoring,
    TestScoringMetric,
};
use knots::{is_source_extension, language_for_file};

fn get_complexity_emoji(complexity: u32) -> &'static str {
    match complexity {
        1..=10 => "😊",   // Smiley - good complexity
        11..=20 => "😐",  // Neutral - okay complexity
        21..=49 => "😠",  // Angry - bad complexity
        _ => "😢",        // Sad - worst complexity (50+)
    }
}

/// Compilation database entry from compile_commands.json
#[derive(Debug, Clone, Deserialize)]
struct CompileCommand {
    #[serde(default)]
    directory: String,
    #[serde(default)]
    _command: String,
    file: String,
    #[serde(default)]
    #[serde(rename = "arguments")]
    _arguments: Vec<String>,
}

/// Filter rules for including/excluding files and functions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterRules {
    /// File path patterns (glob-style, supports negation with !)
    #[serde(default)]
    pub file_patterns: Vec<String>,

    /// Function name patterns (regex)
    #[serde(default)]
    pub function_patterns: Vec<String>,

    /// Minimum complexity threshold (inclusive)
    #[serde(default)]
    pub min_complexity: Option<u32>,

    /// Maximum complexity threshold (inclusive)
    #[serde(default)]
    pub max_complexity: Option<u32>,
}

impl FilterRules {
    /// Load filter rules from a JSON file
    fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read filter file: {}", path.display()))?;
        let rules: FilterRules = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse filter JSON: {}", path.display()))?;
        Ok(rules)
    }

    /// Check if a file path matches the patterns
    fn matches_file(&self, file_path: &str) -> bool {
        if self.file_patterns.is_empty() {
            return true;
        }

        let mut included = false;
        let mut excluded = false;

        for pattern in &self.file_patterns {
            if let Some(neg_pattern) = pattern.strip_prefix('!') {
                // Negation pattern - exclude
                if glob_match(neg_pattern, file_path) {
                    excluded = true;
                }
            } else {
                // Include pattern
                if glob_match(pattern, file_path) {
                    included = true;
                }
            }
        }

        // If we have include patterns, file must match at least one
        // Then check if it's explicitly excluded
        if !self.file_patterns.iter().any(|p| !p.starts_with('!')) {
            // No positive patterns, only negative ones
            !excluded
        } else {
            included && !excluded
        }
    }

    /// Check if a function name matches the patterns
    fn matches_function(&self, function_name: &str) -> bool {
        if self.function_patterns.is_empty() {
            return true;
        }

        for pattern in &self.function_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(function_name) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if complexity is within bounds
    fn matches_complexity(&self, complexity: u32) -> bool {
        if let Some(min) = self.min_complexity {
            if complexity < min {
                return false;
            }
        }
        if let Some(max) = self.max_complexity {
            if complexity > max {
                return false;
            }
        }
        true
    }
}

/// Simple glob matching (supports * and **)
fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern_regex = pattern
        .replace(".", "\\.")
        .replace("**", "<!DOUBLESTAR!>")
        .replace("*", "[^/]*")
        .replace("<!DOUBLESTAR!>", ".*");

    if let Ok(re) = Regex::new(&format!("^{}$", pattern_regex)) {
        re.is_match(path)
    } else {
        false
    }
}

#[derive(Parser, Debug)]
#[command(name = "knots")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Analyzes C/C++ code complexity with visual indicators: 😊 (1-10), 😐 (11-20), 😠 (21-49), 😢 (50+)", long_about = None)]
struct Args {
    /// Path to the C/C++ file or directory to analyze
    #[arg(value_name = "FILE", required_unless_present = "compile_commands")]
    file: Option<PathBuf>,

    /// Recursively process all C/C++ source files in directories
    #[arg(short, long)]
    recursive: bool,

    /// Use compile_commands.json to get list of files to analyze
    #[arg(long, value_name = "FILE", conflicts_with = "file")]
    compile_commands: Option<PathBuf>,

    /// Show detailed per-function analysis
    #[arg(short, long)]
    verbose: bool,

    /// Show testability matrix categorization
    #[arg(short, long)]
    matrix: bool,

    /// Include filter rules from JSON file (whitelist files/functions)
    #[arg(long, value_name = "FILE")]
    include: Option<PathBuf>,

    /// Exclude filter rules from JSON file (blacklist files/functions)
    #[arg(long, value_name = "FILE")]
    exclude: Option<PathBuf>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output (default)
    Text,
    /// SARIF 2.1.0 JSON output for editor/CI integration
    Sarif,
}

/// Parse a source file into a tree-sitter Tree, selecting the grammar by extension.
/// @brief Parse C/C++ source file into AST
/// @version 1
fn parse_file(file: &Path, source_code: &str) -> Result<Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language_for_file(file))
        .context("Failed to set language")?;
    parser
        .parse(source_code, None)
        .with_context(|| format!("Failed to parse {}", file.display()))
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load filter rules
    let include_rules = if let Some(path) = &args.include {
        Some(FilterRules::from_file(path)?)
    } else {
        None
    };

    let exclude_rules = if let Some(path) = &args.exclude {
        Some(FilterRules::from_file(path)?)
    } else {
        None
    };

    // Collect files to process
    let files = if let Some(compile_commands_path) = &args.compile_commands {
        // Load files from compile_commands.json
        load_compile_commands(compile_commands_path, &include_rules, &exclude_rules)?
    } else if let Some(file_path) = &args.file {
        // Use regular file/directory path
        collect_files(file_path, args.recursive, &include_rules, &exclude_rules)?
    } else {
        anyhow::bail!("Either FILE or --compile-commands must be specified");
    };

    // SARIF mode: collect metrics across all files and emit a SARIF 2.1.0 log.
    // This bypasses text/matrix output so the JSON is the only thing on stdout.
    if args.format == OutputFormat::Sarif {
        let (all_metrics, _skipped_files) = collect_all_metrics(&files, &include_rules, &exclude_rules);
        emit_sarif(&all_metrics)?;
        return Ok(());
    }

    // For matrix mode
    if args.matrix {
        let mut all_metrics = Vec::new();
        let mut skipped_files = 0;

        for file in &files {
            let source_code = match fs::read_to_string(file) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Warning: Skipping {}: {}", file.display(), e);
                    skipped_files += 1;
                    continue;
                }
            };

            let tree = match parse_file(file, &source_code) {
                Ok(t) => t,
                Err(_) => {
                    let hint = if file.extension().and_then(|e| e.to_str()) == Some("h") {
                        " (if this file contains C++, rename to .hpp)"
                    } else {
                        ""
                    };
                    eprintln!("Warning: Failed to parse {}{}", file.display(), hint);
                    skipped_files += 1;
                    continue;
                }
            };

            let metrics = collect_function_metrics(&tree, &source_code, file.to_str().unwrap_or(""), &include_rules, &exclude_rules);
            all_metrics.extend(metrics);
        }

        if all_metrics.is_empty() {
            anyhow::bail!("No functions found in any files (skipped {} files)", skipped_files);
        }

        display_testability_matrix(&all_metrics, files.len(), skipped_files);
        return Ok(());
    }

    // For single file mode, use traditional output
    if files.len() == 1 {
        let file = &files[0];
        let source_code = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let tree = parse_file(file, &source_code)?;

        analyze_code(&tree, &source_code, args.verbose, &include_rules, &exclude_rules)?;
        return Ok(());
    }

    // For recursive mode with multiple files: collect all metrics, write report, show summary
    let mut all_metrics = Vec::new();
    let mut skipped_files = 0;

    for file in &files {
        let source_code = match fs::read_to_string(file) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Warning: Skipping {}: {}", file.display(), e);
                skipped_files += 1;
                continue;
            }
        };

        let tree = match parse_file(file, &source_code) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("Warning: Failed to parse {}", file.display());
                skipped_files += 1;
                continue;
            }
        };

        let metrics = collect_function_metrics(&tree, &source_code, file.to_str().unwrap_or(""), &include_rules, &exclude_rules);
        all_metrics.extend(metrics);
    }

    if all_metrics.is_empty() {
        anyhow::bail!("No functions found in any files (skipped {} files)", skipped_files);
    }

    // Write detailed report to file
    write_detailed_report(&all_metrics, args.verbose)?;

    // Display summary with top 5 worst functions and totals/averages
    display_recursive_summary(&all_metrics, files.len(), skipped_files);

    Ok(())
}

/// Load file paths from compile_commands.json
fn load_compile_commands(
    compile_commands_path: &Path,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> Result<Vec<PathBuf>> {
    let content = fs::read_to_string(compile_commands_path)
        .with_context(|| format!("Failed to read compile_commands.json: {}", compile_commands_path.display()))?;

    let commands: Vec<CompileCommand> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse compile_commands.json: {}", compile_commands_path.display()))?;

    let mut files = Vec::new();

    for cmd in commands {
        let file_path = PathBuf::from(&cmd.file);

        // Only process C/C++ source files
        if let Some(ext) = file_path.extension() {
            if is_source_extension(ext) {
                let file_str = file_path.to_string_lossy();
                if should_process_file(&file_str, include_rules, exclude_rules) {
                    // Use absolute path if available, otherwise relative
                    if file_path.is_absolute() {
                        files.push(file_path);
                    } else {
                        // Try to make it absolute using the directory from compile command
                        let abs_path = if !cmd.directory.is_empty() {
                            PathBuf::from(&cmd.directory).join(&file_path)
                        } else {
                            file_path.clone()
                        };

                        if abs_path.exists() {
                            files.push(abs_path);
                        } else if file_path.exists() {
                            files.push(file_path);
                        }
                    }
                }
            }
        }
    }

    if files.is_empty() {
        anyhow::bail!("No C/C++ source files found in compile_commands.json");
    }

    Ok(files)
}

/// Collect files to process based on the path and recursive flag
fn collect_files(
    path: &PathBuf,
    recursive: bool,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        // Single file mode
        let file_str = path.to_string_lossy();
        if should_process_file(&file_str, include_rules, exclude_rules) {
            files.push(path.clone());
        }
    } else if path.is_dir() {
        if !recursive {
            anyhow::bail!(
                "Path '{}' is a directory. Use -r/--recursive to process directories recursively.",
                path.display()
            );
        }

        // Recursive directory mode - only scan .c files by default
        // (headers often contain inline/vendor code)
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if is_source_extension(ext) {
                        let file_str = file_path.to_string_lossy();
                        if should_process_file(&file_str, include_rules, exclude_rules) {
                            files.push(file_path.to_path_buf());
                        }
                    }
                }
            }
        }

        if files.is_empty() {
            anyhow::bail!("No C/C++ source files found in directory: {}", path.display());
        }
    } else {
        anyhow::bail!("Path '{}' does not exist", path.display());
    }

    Ok(files)
}

/// Check if a file should be processed based on include/exclude rules
fn should_process_file(
    file_path: &str,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> bool {
    // Check include rules first (whitelist)
    if let Some(rules) = include_rules {
        if !rules.matches_file(file_path) {
            return false;
        }
    }

    // Check exclude rules (blacklist) - if it matches exclude, DON'T process
    if let Some(rules) = exclude_rules {
        if rules.matches_file(file_path) {
            return false;
        }
    }

    true
}

/// Collect function metrics from a file
fn collect_function_metrics(
    tree: &Tree,
    source_code: &str,
    file_path: &str,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> Vec<FunctionMetrics> {
    let root_node = tree.root_node();
    let mut cursor = root_node.walk();
    let mut metrics = Vec::new();

    visit_functions(&mut cursor, source_code, &mut |node, src| {
        if let Some(name) = get_function_name(node, src) {
            let mccabe = calculate_mccabe_complexity(node, src.as_bytes());
            let cognitive = calculate_cognitive_complexity(node, src.as_bytes());
            let nesting = calculate_nesting_depth(node);
            let sloc = calculate_sloc(node, src.as_bytes());
            let abc = calculate_abc_complexity(node, src.as_bytes());
            let abc_magnitude = abc.magnitude();
            let return_count = calculate_return_count(node);
            let test_scoring = calculate_test_scoring(node, src.as_bytes());

            let max_complexity = std::cmp::max(mccabe, cognitive);

            // Apply filter rules
            if should_process_function(&name, max_complexity, include_rules, exclude_rules) {
                let start_line = (node.start_position().row as u32) + 1;
                let end_line = (node.end_position().row as u32) + 1;
                metrics.push(FunctionMetrics {
                    name,
                    file_path: file_path.to_string(),
                    start_line,
                    end_line,
                    mccabe,
                    cognitive,
                    nesting,
                    sloc,
                    abc_magnitude,
                    return_count,
                    test_scoring,
                });
            }
        }
    });

    metrics
}

/// Check if a function should be processed based on include/exclude rules
fn should_process_function(
    function_name: &str,
    complexity: u32,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> bool {
    // Check include rules first (whitelist)
    if let Some(rules) = include_rules {
        if !rules.matches_function(function_name) {
            return false;
        }
        if !rules.matches_complexity(complexity) {
            return false;
        }
    }

    // Check exclude rules (blacklist) - if it matches exclude, DON'T process
    // Only apply function/complexity filters if they're actually specified
    if let Some(rules) = exclude_rules {
        let matches_func = !rules.function_patterns.is_empty() && rules.matches_function(function_name);
        let matches_complexity = (rules.min_complexity.is_some() || rules.max_complexity.is_some()) && rules.matches_complexity(complexity);

        // If no function patterns specified, only check complexity
        // If no complexity bounds specified, only check function patterns
        // If both specified, require both to match
        let should_exclude = if rules.function_patterns.is_empty() && rules.min_complexity.is_none() && rules.max_complexity.is_none() {
            // No function-level filters, don't exclude based on function criteria
            false
        } else if rules.function_patterns.is_empty() {
            // Only complexity filter
            matches_complexity
        } else if rules.min_complexity.is_none() && rules.max_complexity.is_none() {
            // Only function pattern filter
            matches_func
        } else {
            // Both specified, require both
            matches_func && matches_complexity
        };

        if should_exclude {
            return false;
        }
    }

    true
}

fn analyze_code(
    tree: &Tree,
    source_code: &str,
    verbose: bool,
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> Result<()> {
    let metrics = collect_function_metrics(tree, source_code, "", include_rules, exclude_rules);

    let mut total_mccabe = 0;
    let mut total_cognitive = 0;
    let mut total_nesting = 0;
    let mut total_sloc = 0;
    let mut total_abc_magnitude = 0.0;
    let mut total_return_count = 0;
    let mut total_test_score: i64 = 0;

    for func in &metrics {
        total_mccabe += func.mccabe;
        total_cognitive += func.cognitive;
        total_nesting += func.nesting;
        total_sloc += func.sloc;
        total_abc_magnitude += func.abc_magnitude;
        total_return_count += func.return_count;
        total_test_score += func.test_scoring.total_score as i64;

        let emoji = get_complexity_emoji(func.max_complexity());

        if verbose {
            println!("Function: {} {}", func.name, emoji);
            println!("  McCabe Complexity: {}", func.mccabe);
            println!("  Cognitive Complexity: {}", func.cognitive);
            println!("  Nesting Depth: {}", func.nesting);
            println!("  SLOC: {}", func.sloc);
            println!("  ABC Magnitude: {:.2}", func.abc_magnitude);
            println!("  Return Count: {}", func.return_count);
            println!("  Test Scoring: {} ({})", func.test_scoring.total_score, func.test_scoring.classification());
            println!("    - Signature: {}", func.test_scoring.signature_score);
            println!("    - Dependency: {}", func.test_scoring.dependency_score);
            println!("    - Observable: {}", func.test_scoring.observable_score);
            println!("    - Implementation: {}", func.test_scoring.implementation_score);
            println!("    - Documentation: {}", func.test_scoring.documentation_score);
            println!("  Max Complexity: {}", func.max_complexity());
            println!();
        } else {
            println!(
                "{} {} (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {})",
                emoji, func.name, func.mccabe, func.cognitive, func.nesting, func.sloc, func.abc_magnitude, func.return_count, func.test_scoring.total_score
            );
        }
    }

    let function_count = metrics.len();

    // Print summary
    println!();
    println!("Summary:");
    println!("  Total Functions: {}", function_count);
    println!("  Total McCabe Complexity: {}", total_mccabe);
    println!("  Total Cognitive Complexity: {}", total_cognitive);
    println!("  Total Nesting Depth: {}", total_nesting);
    println!("  Total SLOC: {}", total_sloc);
    println!("  Total ABC Magnitude: {:.2}", total_abc_magnitude);
    println!("  Total Return Count: {}", total_return_count);
    println!("  Total Test Score: {}", total_test_score);

    if function_count > 0 {
        println!("  Average McCabe Complexity: {:.2}", total_mccabe as f64 / function_count as f64);
        println!("  Average Cognitive Complexity: {:.2}", total_cognitive as f64 / function_count as f64);
        println!("  Average Nesting Depth: {:.2}", total_nesting as f64 / function_count as f64);
        println!("  Average SLOC: {:.2}", total_sloc as f64 / function_count as f64);
        println!("  Average ABC Magnitude: {:.2}", total_abc_magnitude / function_count as f64);
        println!("  Average Return Count: {:.2}", total_return_count as f64 / function_count as f64);
        println!("  Average Test Score: {:.2}", total_test_score as f64 / function_count as f64);
    }

    Ok(())
}

/// Collect FunctionMetrics across multiple files, skipping unreadable/unparseable ones.
/// Returns (metrics, skipped_file_count). Used by SARIF mode.
fn collect_all_metrics(
    files: &[PathBuf],
    include_rules: &Option<FilterRules>,
    exclude_rules: &Option<FilterRules>,
) -> (Vec<FunctionMetrics>, usize) {
    let mut all_metrics = Vec::new();
    let mut skipped = 0;

    for file in files {
        let source_code = match fs::read_to_string(file) {
            Ok(code) => code,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let tree = match parse_file(file, &source_code) {
            Ok(t) => t,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        let metrics = collect_function_metrics(
            &tree,
            &source_code,
            file.to_str().unwrap_or(""),
            include_rules,
            exclude_rules,
        );
        all_metrics.extend(metrics);
    }

    (all_metrics, skipped)
}

/// SARIF level for a complexity bucket. Mirrors the emoji buckets:
/// 1-10 healthy (no result emitted), 11-20 note, 21-49 warning, 50+ error.
fn sarif_level(complexity: u32) -> &'static str {
    match complexity {
        0..=10 => "none",
        11..=20 => "note",
        21..=49 => "warning",
        _ => "error",
    }
}

/// Emit a SARIF 2.1.0 log to stdout describing functions whose max(McCabe, Cognitive)
/// exceeds the healthy threshold (>10). One result per offending function.
fn emit_sarif(all_metrics: &[FunctionMetrics]) -> Result<()> {
    use serde_json::json;

    let rules = json!([
        {
            "id": "knots/high-complexity",
            "name": "HighComplexity",
            "shortDescription": { "text": "Function exceeds complexity threshold" },
            "fullDescription": {
                "text": "Reports functions whose McCabe or cognitive complexity exceeds the healthy threshold (10). Severity escalates at 21 (warning) and 50 (error)."
            },
            "defaultConfiguration": { "level": "note" },
            "helpUri": "https://github.com/brandon-arrendondo/knots"
        }
    ]);

    let mut results = Vec::new();
    for func in all_metrics {
        let max = std::cmp::max(func.mccabe, func.cognitive);
        if max <= 10 {
            continue;
        }

        let uri = path_to_sarif_uri(&func.file_path);
        let message = format!(
            "{} has high complexity (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {})",
            func.name,
            func.mccabe,
            func.cognitive,
            func.nesting,
            func.sloc,
            func.abc_magnitude,
            func.return_count,
            func.test_scoring.total_score
        );

        results.push(json!({
            "ruleId": "knots/high-complexity",
            "level": sarif_level(max),
            "message": { "text": message },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": { "uri": uri },
                    "region": {
                        "startLine": func.start_line,
                        "endLine": func.end_line
                    }
                },
                "logicalLocations": [{
                    "name": func.name,
                    "kind": "function"
                }]
            }],
            "properties": {
                "mccabe": func.mccabe,
                "cognitive": func.cognitive,
                "nesting": func.nesting,
                "sloc": func.sloc,
                "abcMagnitude": func.abc_magnitude,
                "returnCount": func.return_count,
                "testScore": func.test_scoring.total_score
            }
        }));
    }

    let log = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "knots",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/brandon-arrendondo/knots",
                    "rules": rules
                }
            },
            "results": results
        }]
    });

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &log).context("Failed to write SARIF JSON")?;
    writeln!(handle)?;
    Ok(())
}

/// Convert a filesystem path string to a relative URI suitable for SARIF
/// artifactLocation. Uses a path relative to the current working directory
/// when possible, otherwise falls back to the raw path.
fn path_to_sarif_uri(file_path: &str) -> String {
    let path = Path::new(file_path);
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    file_path.replace('\\', "/")
}

/// Write detailed report to report.txt for recursive analysis
fn write_detailed_report(all_metrics: &[FunctionMetrics], verbose: bool) -> Result<()> {
    let mut file = fs::File::create("report.txt")
        .context("Failed to create report.txt")?;

    for func in all_metrics {
        let emoji = get_complexity_emoji(func.max_complexity());

        if verbose {
            writeln!(file, "Function: {} {} [{}]", func.name, emoji, func.file_path)?;
            writeln!(file, "  McCabe Complexity: {}", func.mccabe)?;
            writeln!(file, "  Cognitive Complexity: {}", func.cognitive)?;
            writeln!(file, "  Nesting Depth: {}", func.nesting)?;
            writeln!(file, "  SLOC: {}", func.sloc)?;
            writeln!(file, "  ABC Magnitude: {:.2}", func.abc_magnitude)?;
            writeln!(file, "  Return Count: {}", func.return_count)?;
            writeln!(file, "  Test Scoring: {} ({})", func.test_scoring.total_score, func.test_scoring.classification())?;
            writeln!(file, "    - Signature: {}", func.test_scoring.signature_score)?;
            writeln!(file, "    - Dependency: {}", func.test_scoring.dependency_score)?;
            writeln!(file, "    - Observable: {}", func.test_scoring.observable_score)?;
            writeln!(file, "    - Implementation: {}", func.test_scoring.implementation_score)?;
            writeln!(file, "    - Documentation: {}", func.test_scoring.documentation_score)?;
            writeln!(file, "  Max Complexity: {}", func.max_complexity())?;
            writeln!(file)?;
        } else {
            writeln!(
                file,
                "{} {} [{}] (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {})",
                emoji, func.name, func.file_path, func.mccabe, func.cognitive, func.nesting, func.sloc, func.abc_magnitude, func.return_count, func.test_scoring.total_score
            )?;
        }
    }

    Ok(())
}

/// Display summary with top 5 worst functions and totals/averages
fn display_recursive_summary(all_metrics: &[FunctionMetrics], total_files: usize, skipped_files: usize) {
    // Sort by worst complexity (max of McCabe and Cognitive)
    let mut sorted = all_metrics.to_vec();
    sorted.sort_by(|a, b| b.max_complexity().cmp(&a.max_complexity()));

    println!("\n=== TOP 5 WORST FUNCTIONS ===\n");
    for (i, func) in sorted.iter().take(5).enumerate() {
        let emoji = get_complexity_emoji(func.max_complexity());
        println!(
            "{}. {} {} [{}]",
            i + 1,
            emoji,
            func.name,
            func.file_path
        );
        println!("   McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {}",
            func.mccabe, func.cognitive, func.nesting, func.sloc, func.abc_magnitude, func.return_count, func.test_scoring.total_score
        );
    }

    // Calculate totals and averages
    let mut total_mccabe: u64 = 0;
    let mut total_cognitive: u64 = 0;
    let mut total_nesting: u64 = 0;
    let mut total_sloc: u64 = 0;
    let mut total_abc_magnitude = 0.0;
    let mut total_return_count: u64 = 0;
    let mut total_test_score: i64 = 0;

    for func in all_metrics {
        total_mccabe += func.mccabe as u64;
        total_cognitive += func.cognitive as u64;
        total_nesting += func.nesting as u64;
        total_sloc += func.sloc as u64;
        total_abc_magnitude += func.abc_magnitude;
        total_return_count += func.return_count as u64;
        total_test_score += func.test_scoring.total_score as i64;
    }

    let function_count = all_metrics.len();

    println!("\n=== TOTALS & AVERAGES ===\n");
    println!("  Total Functions: {}", function_count);
    println!("  Total McCabe Complexity: {}", total_mccabe);
    println!("  Total Cognitive Complexity: {}", total_cognitive);
    println!("  Total Nesting Depth: {}", total_nesting);
    println!("  Total SLOC: {}", total_sloc);
    println!("  Total ABC Magnitude: {:.2}", total_abc_magnitude);
    println!("  Total Return Count: {}", total_return_count);
    println!("  Total Test Score: {}", total_test_score);

    if function_count > 0 {
        println!();
        println!("  Average McCabe Complexity: {:.2}", total_mccabe as f64 / function_count as f64);
        println!("  Average Cognitive Complexity: {:.2}", total_cognitive as f64 / function_count as f64);
        println!("  Average Nesting Depth: {:.2}", total_nesting as f64 / function_count as f64);
        println!("  Average SLOC: {:.2}", total_sloc as f64 / function_count as f64);
        println!("  Average ABC Magnitude: {:.2}", total_abc_magnitude / function_count as f64);
        println!("  Average Return Count: {:.2}", total_return_count as f64 / function_count as f64);
        println!("  Average Test Score: {:.2}", total_test_score as f64 / function_count as f64);
    }

    println!("\nDetailed per-function output written to report.txt");
    println!("\n=== FILES PROCESSED ===\n");
    println!("  Total files found: {}", total_files);
    println!("  Successfully processed: {}", total_files - skipped_files);
    if skipped_files > 0 {
        println!("  Skipped (encoding/parse errors): {}", skipped_files);
    }
}

#[derive(Debug, Clone)]
struct FunctionMetrics {
    name: String,
    file_path: String,
    start_line: u32,
    end_line: u32,
    mccabe: u32,
    cognitive: u32,
    nesting: u32,
    sloc: u32,
    abc_magnitude: f64,
    return_count: u32,
    test_scoring: TestScoringMetric,
}

impl FunctionMetrics {
    fn max_complexity(&self) -> u32 {
        std::cmp::max(self.mccabe, self.cognitive)
    }
}

/// Display testability matrix for all functions
fn display_testability_matrix(all_metrics: &[FunctionMetrics], total_files: usize, skipped_files: usize) {
    // Categorize functions into quadrants
    let mut quick_wins = Vec::new();
    let mut invest_tests = Vec::new();
    let mut add_docs = Vec::new();
    let mut refactor = Vec::new();

    for func in all_metrics {
        let low_complexity = func.mccabe <= 10;
        let easy_to_test = func.test_scoring.total_score <= 10;

        match (low_complexity, easy_to_test) {
            (true, true) => quick_wins.push(func),
            (false, true) => invest_tests.push(func),
            (true, false) => add_docs.push(func),
            (false, false) => refactor.push(func),
        }
    }

    // Print matrix results
    println!("\n=== TESTABILITY MATRIX ===\n");

    println!("📊 QUICK WINS (Low Complexity, Easy to Test) - Automate!");
    println!("=========================================================");
    if quick_wins.is_empty() {
        println!("  (none)");
    } else {
        for func in &quick_wins {
            if func.file_path.is_empty() {
                println!("  ✓ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
            } else {
                println!("  ✓ {} [{}] (McCabe: {}, TestScore: {})", func.name, func.file_path, func.mccabe, func.test_scoring.total_score);
            }
        }
    }
    println!();

    println!("🎯 INVEST IN TESTS (High Complexity, Easy to Test)");
    println!("==================================================");
    if invest_tests.is_empty() {
        println!("  (none)");
    } else {
        for func in &invest_tests {
            if func.file_path.is_empty() {
                println!("  → {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
            } else {
                println!("  → {} [{}] (McCabe: {}, TestScore: {})", func.name, func.file_path, func.mccabe, func.test_scoring.total_score);
            }
        }
    }
    println!();

    println!("📝 ADD DOCS (Low Complexity, Hard to Test)");
    println!("===========================================");
    if add_docs.is_empty() {
        println!("  (none)");
    } else {
        for func in &add_docs {
            if func.file_path.is_empty() {
                println!("  ⚠ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
            } else {
                println!("  ⚠ {} [{}] (McCabe: {}, TestScore: {})", func.name, func.file_path, func.mccabe, func.test_scoring.total_score);
            }
        }
    }
    println!();

    println!("🚨 REFACTOR (High Complexity, Hard to Test) - HIGH RISK!");
    println!("========================================================");
    if refactor.is_empty() {
        println!("  (none)");
    } else {
        for func in &refactor {
            if func.file_path.is_empty() {
                println!("  ⛔ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
            } else {
                println!("  ⛔ {} [{}] (McCabe: {}, TestScore: {})", func.name, func.file_path, func.mccabe, func.test_scoring.total_score);
            }
        }
    }
    println!();

    // Print summary
    println!("=== SUMMARY ===\n");
    println!("  Quick Wins:    {} functions", quick_wins.len());
    println!("  Invest Tests:  {} functions", invest_tests.len());
    println!("  Add Docs:      {} functions", add_docs.len());
    println!("  Refactor:      {} functions", refactor.len());
    println!("  Total:         {} functions", all_metrics.len());

    if total_files > 1 {
        println!();
        println!("=== FILES PROCESSED ===\n");
        println!("  Total files found: {}", total_files);
        println!("  Successfully processed: {}", total_files - skipped_files);
        if skipped_files > 0 {
            println!("  Skipped (encoding/parse errors): {}", skipped_files);
        }
    }
}

fn visit_functions<F>(cursor: &mut TreeCursor, source_code: &str, callback: &mut F)
where
    F: FnMut(Node, &str),
{
    let node = cursor.node();

    if node.kind() == "function_definition" {
        callback(node, source_code);
    }

    if cursor.goto_first_child() {
        loop {
            visit_functions(cursor, source_code, callback);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn get_function_name(node: Node, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            return get_declarator_name(child, source_code);
        } else if child.kind() == "pointer_declarator" {
            // For functions returning pointers, the function_declarator is nested inside
            if let Some(name) = get_function_name_from_declarator(child, source_code) {
                return Some(name);
            }
        }
    }

    None
}

fn get_function_name_from_declarator(node: Node, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            return get_declarator_name(child, source_code);
        } else if child.kind() == "pointer_declarator" {
            if let Some(name) = get_function_name_from_declarator(child, source_code) {
                return Some(name);
            }
        }
    }

    None
}

fn get_declarator_name(node: Node, source_code: &str) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "qualified_identifier" | "destructor_name"
            | "operator_name" | "field_identifier" => {
                return Some(child.utf8_text(source_code.as_bytes()).ok()?.to_string());
            }
            "pointer_declarator" | "function_declarator" => {
                if let Some(name) = get_declarator_name(child, source_code) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse C++ code and collect discovered function names via visit_functions + get_function_name.
    fn discover_cpp_functions(code: &str) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_cpp::language())
            .unwrap();
        let tree = parser.parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        let mut names = Vec::new();
        visit_functions(&mut cursor, code, &mut |node, src| {
            if let Some(name) = get_function_name(node, src) {
                names.push(name);
            }
        });
        names
    }

    #[test]
    fn test_cpp_discover_namespace_function() {
        let names = discover_cpp_functions(
            r#"namespace myns { void func() { int x = 0; } }"#,
        );
        assert_eq!(names, vec!["func"]);
    }

    #[test]
    fn test_cpp_discover_class_method() {
        let names = discover_cpp_functions(
            r#"class Foo { void method() { int x = 0; } };"#,
        );
        assert_eq!(names, vec!["method"]);
    }

    #[test]
    fn test_cpp_discover_template_function() {
        let names = discover_cpp_functions(
            r#"template<typename T> T add(T a, T b) { return a + b; }"#,
        );
        assert_eq!(names, vec!["add"]);
    }

    #[test]
    fn test_cpp_discover_qualified_name() {
        let names = discover_cpp_functions(
            r#"void Foo::bar() { int x = 0; }"#,
        );
        assert_eq!(names, vec!["Foo::bar"]);
    }

    #[test]
    fn test_cpp_discover_operator() {
        let names = discover_cpp_functions(
            r#"Foo operator+(Foo a, Foo b) { return a; }"#,
        );
        assert_eq!(names.len(), 1);
        assert!(names[0].contains("operator+"), "Expected operator+, got: {}", names[0]);
    }

    #[test]
    fn test_cpp_discover_destructor() {
        let names = discover_cpp_functions(
            r#"class Foo { ~Foo() { int x = 0; } };"#,
        );
        assert_eq!(names, vec!["~Foo"]);
    }
}
