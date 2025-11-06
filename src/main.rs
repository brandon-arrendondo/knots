use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use tree_sitter::{Node, Tree, TreeCursor};

mod complexity;
use complexity::{
    calculate_abc_complexity, calculate_cognitive_complexity, calculate_mccabe_complexity,
    calculate_nesting_depth, calculate_return_count, calculate_sloc, calculate_test_scoring,
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
    /// Path to the C file to analyze
    #[arg(value_name = "FILE")]
    file: PathBuf,

    /// Show detailed per-function analysis
    #[arg(short, long)]
    verbose: bool,

    /// Show testability matrix categorization
    #[arg(short, long)]
    matrix: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read the C file
    let source_code = fs::read_to_string(&args.file)
        .with_context(|| format!("Failed to read file: {}", args.file.display()))?;

    // Parse the C code using tree-sitter
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c::language())
        .context("Failed to set C language")?;

    let tree = parser
        .parse(&source_code, None)
        .context("Failed to parse C code")?;

    // Analyze the code
    if args.matrix {
        analyze_matrix(&tree, &source_code)?;
    } else {
        analyze_code(&tree, &source_code, args.verbose)?;
    }

    Ok(())
}

fn analyze_code(tree: &Tree, source_code: &str, verbose: bool) -> Result<()> {
    let root_node = tree.root_node();
    let mut cursor = root_node.walk();

    let mut total_mccabe = 0;
    let mut total_cognitive = 0;
    let mut total_nesting = 0;
    let mut total_sloc = 0;
    let mut total_abc_magnitude = 0.0;
    let mut total_return_count = 0;
    let mut total_test_score = 0;
    let mut function_count = 0;

    // Find all function definitions
    visit_functions(&mut cursor, source_code, &mut |node, src| {
        if let Some(name) = get_function_name(node, src) {
            function_count += 1;

            let mccabe = calculate_mccabe_complexity(node, src.as_bytes());
            let cognitive = calculate_cognitive_complexity(node, src.as_bytes());
            let nesting = calculate_nesting_depth(node);
            let sloc = calculate_sloc(node, src.as_bytes());
            let abc = calculate_abc_complexity(node, src.as_bytes());
            let abc_magnitude = abc.magnitude();
            let return_count = calculate_return_count(node);
            let test_scoring = calculate_test_scoring(node, src.as_bytes());

            total_mccabe += mccabe;
            total_cognitive += cognitive;
            total_nesting += nesting;
            total_sloc += sloc;
            total_abc_magnitude += abc_magnitude;
            total_return_count += return_count;
            total_test_score += test_scoring.total_score as i64;

            // Always show per-function analysis with emojis
            let max_complexity = std::cmp::max(mccabe, cognitive);
            let emoji = get_complexity_emoji(max_complexity);

            if verbose {
                println!("Function: {} {}", name, emoji);
                println!("  McCabe Complexity: {}", mccabe);
                println!("  Cognitive Complexity: {}", cognitive);
                println!("  Nesting Depth: {}", nesting);
                println!("  SLOC: {}", sloc);
                println!("  ABC: <{},{},{}> (magnitude: {:.2})", abc.assignments, abc.branches, abc.conditions, abc_magnitude);
                println!("  Return Count: {}", return_count);
                println!("  Test Scoring: {} ({})", test_scoring.total_score, test_scoring.classification());
                println!("    - Signature: {}", test_scoring.signature_score);
                println!("    - Dependency: {}", test_scoring.dependency_score);
                println!("    - Observable: {}", test_scoring.observable_score);
                println!("    - Implementation: {}", test_scoring.implementation_score);
                println!("    - Documentation: {}", test_scoring.documentation_score);
                println!("  Max Complexity: {}", max_complexity);
                println!();
            } else {
                println!(
                    "{} {} (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {})",
                    emoji, name, mccabe, cognitive, nesting, sloc, abc_magnitude, return_count, test_scoring.total_score
                );
            }
        }
    });

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

#[derive(Debug)]
struct FunctionMetrics {
    name: String,
    mccabe: u32,
    test_score: i32,
}

fn analyze_matrix(tree: &Tree, source_code: &str) -> Result<()> {
    let root_node = tree.root_node();
    let mut cursor = root_node.walk();

    let mut functions: Vec<FunctionMetrics> = Vec::new();

    // Collect all function metrics
    visit_functions(&mut cursor, source_code, &mut |node, src| {
        if let Some(name) = get_function_name(node, src) {
            let mccabe = calculate_mccabe_complexity(node, src.as_bytes());
            let test_scoring = calculate_test_scoring(node, src.as_bytes());

            functions.push(FunctionMetrics {
                name,
                mccabe,
                test_score: test_scoring.total_score,
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
        let easy_to_test = func.test_score <= 10;

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
            println!("  ✓ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_score);
        }
    }
    println!();

    println!("🎯 INVEST IN TESTS (High Complexity, Easy to Test)");
    println!("==================================================");
    if invest_tests.is_empty() {
        println!("  (none)");
    } else {
        for func in &invest_tests {
            println!("  → {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_score);
        }
    }
    println!();

    println!("📝 ADD DOCS (Low Complexity, Hard to Test)");
    println!("===========================================");
    if add_docs.is_empty() {
        println!("  (none)");
    } else {
        for func in &add_docs {
            println!("  ⚠ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_score);
        }
    }
    println!();

    println!("🚨 REFACTOR (High Complexity, Hard to Test) - HIGH RISK!");
    println!("========================================================");
    if refactor.is_empty() {
        println!("  (none)");
    } else {
        for func in &refactor {
            println!("  ⛔ {} (McCabe: {}, TestScore: {})", func.name, func.mccabe, func.test_score);
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
