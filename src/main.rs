use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tree_sitter::{Node, Tree, TreeCursor};
use walkdir::WalkDir;

mod complexity;
use complexity::{
    calculate_abc_complexity, calculate_cognitive_complexity, calculate_mccabe_complexity,
    calculate_nesting_depth, calculate_return_count, calculate_sloc, calculate_test_scoring,
    TestScoringMetric,
};

fn get_complexity_emoji(complexity: u32) -> &'static str {
    match complexity {
        1..=10 => "😊",   // Smiley - good complexity
        11..=20 => "😐",  // Neutral - okay complexity
        21..=49 => "😠",  // Angry - bad complexity
        _ => "😢",        // Sad - worst complexity (50+)
    }
}

#[derive(Parser, Debug)]
#[command(name = "knots")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Analyzes C code complexity with visual indicators: 😊 (1-10), 😐 (11-20), 😠 (21-49), 😢 (50+)", long_about = None)]
struct Args {
    /// Path to the C file or directory to analyze
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Recursively process all C files in directories
    #[arg(short, long)]
    recursive: bool,

    /// Show detailed per-function analysis
    #[arg(short, long)]
    verbose: bool,

    /// Show testability matrix categorization
    #[arg(short, long)]
    matrix: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Collect files to process
    let files = collect_files(&args.file, args.recursive)?;

    // For matrix mode, always use the old behavior (per-file output)
    if args.matrix {
        for file in &files {
            if files.len() > 1 {
                println!("\n=== {} ===", file.display());
            }

            let source_code = match fs::read_to_string(file) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Warning: Skipping {}: {}", file.display(), e);
                    continue;
                }
            };

            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&tree_sitter_c::language())
                .context("Failed to set C language")?;

            let tree = parser
                .parse(&source_code, None)
                .with_context(|| format!("Failed to parse C code in {}", file.display()))?;

            analyze_matrix(&tree, &source_code)?;
        }
        return Ok(());
    }

    // For single file mode, use traditional output
    if files.len() == 1 {
        let file = &files[0];
        let source_code = fs::read_to_string(file)
            .with_context(|| format!("Failed to read file: {}", file.display()))?;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_c::language())
            .context("Failed to set C language")?;

        let tree = parser
            .parse(&source_code, None)
            .with_context(|| format!("Failed to parse C code in {}", file.display()))?;

        analyze_code(&tree, &source_code, args.verbose)?;
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

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_c::language())
            .context("Failed to set C language")?;

        let tree = match parser.parse(&source_code, None) {
            Some(t) => t,
            None => {
                eprintln!("Warning: Failed to parse {}", file.display());
                skipped_files += 1;
                continue;
            }
        };

        let metrics = collect_function_metrics(&tree, &source_code, file.to_str().unwrap_or(""));
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

/// Collect files to process based on the path and recursive flag
fn collect_files(path: &PathBuf, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        // Single file mode
        files.push(path.clone());
    } else if path.is_dir() {
        if !recursive {
            anyhow::bail!(
                "Path '{}' is a directory. Use -r/--recursive to process directories recursively.",
                path.display()
            );
        }

        // Recursive directory mode
        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "c" || ext == "h" {
                        files.push(file_path.to_path_buf());
                    }
                }
            }
        }

        if files.is_empty() {
            anyhow::bail!("No C files (.c or .h) found in directory: {}", path.display());
        }
    } else {
        anyhow::bail!("Path '{}' does not exist", path.display());
    }

    Ok(files)
}

/// Collect function metrics from a file
fn collect_function_metrics(tree: &Tree, source_code: &str, file_path: &str) -> Vec<FunctionMetrics> {
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

            metrics.push(FunctionMetrics {
                name,
                file_path: file_path.to_string(),
                mccabe,
                cognitive,
                nesting,
                sloc,
                abc_magnitude,
                return_count,
                test_scoring,
            });
        }
    });

    metrics
}

fn analyze_code(tree: &Tree, source_code: &str, verbose: bool) -> Result<()> {
    let metrics = collect_function_metrics(tree, source_code, "");

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

fn analyze_matrix(tree: &Tree, source_code: &str) -> Result<()> {
    let root_node = tree.root_node();
    let mut cursor = root_node.walk();

    let mut functions: Vec<FunctionMetrics> = Vec::new();

    // Collect all function metrics
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

            functions.push(FunctionMetrics {
                name,
                file_path: String::new(),
                mccabe,
                cognitive,
                nesting,
                sloc,
                abc_magnitude,
                return_count,
                test_scoring,
            });
        }
    });

    // Categorize functions into quadrants
    let mut quick_wins = Vec::new();
    let mut invest_tests = Vec::new();
    let mut add_docs = Vec::new();
    let mut refactor = Vec::new();

    for func in functions {
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
    println!("Function Testability Matrix");
    println!("===========================");
    println!();

    println!("📊 QUICK WINS (Low Complexity, Easy to Test) - Automate!");
    println!("=========================================================");
    if quick_wins.is_empty() {
        println!("  (none)");
    } else {
        for func in &quick_wins {
            println!("  ✓ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
        }
    }
    println!();

    println!("🎯 INVEST IN TESTS (High Complexity, Easy to Test)");
    println!("==================================================");
    if invest_tests.is_empty() {
        println!("  (none)");
    } else {
        for func in &invest_tests {
            println!("  → {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
        }
    }
    println!();

    println!("📝 ADD DOCS (Low Complexity, Hard to Test)");
    println!("===========================================");
    if add_docs.is_empty() {
        println!("  (none)");
    } else {
        for func in &add_docs {
            println!("  ⚠ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
        }
    }
    println!();

    println!("🚨 REFACTOR (High Complexity, Hard to Test) - HIGH RISK!");
    println!("========================================================");
    if refactor.is_empty() {
        println!("  (none)");
    } else {
        for func in &refactor {
            println!("  ⛔ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_scoring.total_score);
        }
    }
    println!();

    // Print summary
    println!("Summary:");
    println!("--------");
    println!("  Quick Wins:    {} functions", quick_wins.len());
    println!("  Invest Tests:  {} functions", invest_tests.len());
    println!("  Add Docs:      {} functions", add_docs.len());
    println!("  Refactor:      {} functions", refactor.len());
    println!("  Total:         {} functions", quick_wins.len() + invest_tests.len() + add_docs.len() + refactor.len());

    Ok(())
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
        if child.kind() == "identifier" {
            return Some(child.utf8_text(source_code.as_bytes()).ok()?.to_string());
        } else if child.kind() == "pointer_declarator" || child.kind() == "function_declarator" {
            if let Some(name) = get_declarator_name(child, source_code) {
                return Some(name);
            }
        }
    }

    None
}
