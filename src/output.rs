use anyhow::{Context, Result};
use serde_json::json;
use std::cmp::Reverse;
use std::fs;
use std::io::Write;
use std::path::Path;

use knots::complexity::calculate_aird_raw;
use knots::FunctionMetrics;

pub(crate) fn get_complexity_emoji(complexity: u32) -> &'static str {
    match complexity {
        1..=10 => "😊",
        11..=20 => "😐",
        21..=49 => "😠",
        _ => "😢",
    }
}

/// `file:line:name` locator for a function, so an editor can jump from any line
/// of human output, not just threshold violations. Matches the format the
/// violation block uses; falls back to `name:line` when the file path is unknown.
pub(crate) fn func_location(func: &FunctionMetrics) -> String {
    if func.file_path.is_empty() {
        format!("{}:{}", func.name, func.start_line)
    } else {
        format!("{}:{}:{}", func.file_path, func.start_line, func.name)
    }
}

pub(crate) fn aird_term(label: &str, value: f64, max: f64, cap_label: &str) -> String {
    let capped = if value >= max { cap_label } else { "" };
    format!("{}: {:.1}/{:.0}{}", label, value, max, capped)
}

pub(crate) fn format_aird_breakdown(func: &FunctionMetrics) -> String {
    let cognitive_contrib = (func.cognitive as f64 / 75.0).min(1.0) * 55.0;
    let sloc_contrib = (func.sloc as f64 / 200.0).min(1.0) * 15.0;
    let nesting_contrib = (func.nesting as f64 / 8.0).min(1.0) * 15.0;
    let test_contrib = (func.test_scoring.total_score.max(0) as f64 / 20.0).min(1.0) * 15.0;
    let doc_contrib = (func.test_scoring.documentation_score.max(0) as f64 / 10.0).min(1.0) * 15.0;
    let coupling_contrib = (func.state_coupling as f64 / 12.0).min(1.0) * 10.0;

    let cog = aird_term("cognitive", cognitive_contrib, 55.0, " [capped]");
    let sloc = aird_term("sloc", sloc_contrib, 15.0, " [capped]");
    let nest = aird_term("nesting", nesting_contrib, 15.0, " [capped]");
    let test = aird_term("test", test_contrib, 15.0, " [capped]");
    let doc = format!("doc: -{:.1}/15", doc_contrib);
    let coup = format!("coupling: +{:.1}/10", coupling_contrib);

    let base = format!(
        "    {}, {}, {}, {}, {}, {}",
        cog, sloc, nest, test, doc, coup
    );

    let cognitive_capped = func.cognitive >= 75;
    let sloc_capped = func.sloc >= 200;
    if cognitive_capped || sloc_capped {
        let raw = calculate_aird_raw(
            func.cognitive,
            func.sloc,
            func.nesting,
            func.test_scoring.total_score,
            func.test_scoring.documentation_score,
            func.state_coupling,
        );
        format!("{}\n    raw AIRD (uncapped): {:.0}", base, raw)
    } else {
        base
    }
}

pub(crate) fn aird_drivers(func: &FunctionMetrics, top_n: usize) -> Vec<(&'static str, i64)> {
    let test_score = func.test_scoring.total_score.max(0);
    let mut comps: [(&'static str, i64, f64); 5] = [
        (
            "cognitive",
            func.cognitive as i64,
            (func.cognitive as f64 / 75.0).min(1.0) * 55.0,
        ),
        (
            "sloc",
            func.sloc as i64,
            (func.sloc as f64 / 200.0).min(1.0) * 15.0,
        ),
        (
            "nesting",
            func.nesting as i64,
            (func.nesting as f64 / 8.0).min(1.0) * 15.0,
        ),
        (
            "test",
            test_score as i64,
            (test_score as f64 / 20.0).min(1.0) * 15.0,
        ),
        (
            "coupling",
            func.state_coupling as i64,
            (func.state_coupling as f64 / 12.0).min(1.0) * 10.0,
        ),
    ];
    comps.sort_by(|a, b| b.2.total_cmp(&a.2));
    comps
        .iter()
        .filter(|(_, _, contrib)| *contrib > 0.0)
        .take(top_n)
        .map(|(label, raw, _)| (*label, *raw))
        .collect()
}

pub(crate) fn aird_tips(func: &FunctionMetrics) -> Vec<String> {
    let mut tips = Vec::new();
    let cognitive_capped = func.cognitive >= 75;
    let sloc_capped = func.sloc >= 200;

    if cognitive_capped && sloc_capped {
        let raw = calculate_aird_raw(
            func.cognitive,
            func.sloc,
            func.nesting,
            func.test_scoring.total_score,
            func.test_scoring.documentation_score,
            func.state_coupling,
        );
        tips.push(format!(
            "    Tip: cognitive and sloc are both capped — incremental extractions won't move the \
             needle until you break through at least one cap (raw AIRD: {:.0} — tracks your \
             true progress while the gate score is pinned). Push for a larger extraction that \
             drops cognitive below 75 or sloc below 200.",
            raw
        ));
    } else if cognitive_capped {
        tips.push(
            "    Tip: cognitive is capped at 75 (contributing full 55 pts). \
             Extract until cognitive drops below 75 to see AIRD improve."
                .to_string(),
        );
    }

    if func.state_coupling > 0 {
        tips.push(
            "    Tip: if early extractions are not reducing AIRD, the function may still be \
             above the cognitive cap. Continue extracting — once cognitive drops below 75 \
             the cognitive term will fall sharply and AIRD will follow."
                .to_string(),
        );
    }

    tips
}

/// A one-line pointer printed under AI-metric violations, expanding the acronyms
/// and pointing at `--explain`. Returns None when no AI metric was violated.
pub(crate) fn ai_metric_pointer(any_aird: bool, any_aicp: bool) -> Option<String> {
    match (any_aird, any_aicp) {
        (true, true) => Some(
            "  AIRD = AI Reasoning Difficulty, AICP = AI Context Pressure — run \
             `knots --explain aird` / `knots --explain aicp` for how to lower them."
                .to_string(),
        ),
        (true, false) => Some(
            "  AIRD = AI Reasoning Difficulty — run `knots --explain aird` for how to lower it."
                .to_string(),
        ),
        (false, true) => Some(
            "  AICP = AI Context Pressure — run `knots --explain aicp` for how to lower it."
                .to_string(),
        ),
        (false, false) => None,
    }
}

pub(crate) fn analyze_code(metrics: &[FunctionMetrics], verbose: bool) -> Result<()> {
    let mut total_mccabe = 0;
    let mut total_cognitive = 0;
    let mut total_nesting = 0;
    let mut total_sloc = 0;
    let mut total_abc_magnitude = 0.0;
    let mut total_return_count = 0;
    let mut total_test_score: i64 = 0;
    let mut total_aird: u64 = 0;
    let mut total_aicp: u64 = 0;

    for func in metrics {
        total_mccabe += func.mccabe;
        total_cognitive += func.cognitive;
        total_nesting += func.nesting;
        total_sloc += func.sloc;
        total_abc_magnitude += func.abc_magnitude;
        total_return_count += func.return_count;
        total_test_score += func.test_scoring.total_score as i64;
        total_aird += func.aird as u64;
        total_aicp += func.aicp as u64;

        let emoji = get_complexity_emoji(func.max_complexity());

        if verbose {
            println!("Function: {} {}", func_location(func), emoji);
            println!("  McCabe Complexity: {}", func.mccabe);
            println!("  Cognitive Complexity: {}", func.cognitive);
            println!("  Nesting Depth: {}", func.nesting);
            println!("  SLOC: {}", func.sloc);
            println!("  ABC Magnitude: {:.2}", func.abc_magnitude);
            println!("  Return Count: {}", func.return_count);
            println!(
                "  Test Scoring: {} ({})",
                func.test_scoring.total_score,
                func.test_scoring.classification()
            );
            println!("    - Signature: {}", func.test_scoring.signature_score);
            println!("    - Dependency: {}", func.test_scoring.dependency_score);
            println!("    - Observable: {}", func.test_scoring.observable_score);
            println!(
                "    - Implementation: {}",
                func.test_scoring.implementation_score
            );
            println!(
                "    - Documentation: {}",
                func.test_scoring.documentation_score
            );
            println!("  AIRD Score: {}", func.aird);
            println!("  AICP Score: {}", func.aicp);
            println!("  External Calls: {}", func.external_calls);
            println!("  State Coupling: {}", func.state_coupling);
            println!("  Max Complexity: {}", func.max_complexity());
            println!();
        } else {
            println!(
                "{} {} (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {}, AIRD: {}, AICP: {}, ExtCalls: {})",
                emoji, func_location(func), func.mccabe, func.cognitive, func.nesting, func.sloc,
                func.abc_magnitude, func.return_count, func.test_scoring.total_score, func.aird,
                func.aicp, func.external_calls
            );
        }
    }

    let function_count = metrics.len();

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
    println!("  Total AIRD Score: {}", total_aird);
    println!("  Total AICP Score: {}", total_aicp);

    if function_count > 0 {
        println!(
            "  Average McCabe Complexity: {:.2}",
            total_mccabe as f64 / function_count as f64
        );
        println!(
            "  Average Cognitive Complexity: {:.2}",
            total_cognitive as f64 / function_count as f64
        );
        println!(
            "  Average Nesting Depth: {:.2}",
            total_nesting as f64 / function_count as f64
        );
        println!(
            "  Average SLOC: {:.2}",
            total_sloc as f64 / function_count as f64
        );
        println!(
            "  Average ABC Magnitude: {:.2}",
            total_abc_magnitude / function_count as f64
        );
        println!(
            "  Average Return Count: {:.2}",
            total_return_count as f64 / function_count as f64
        );
        println!(
            "  Average Test Score: {:.2}",
            total_test_score as f64 / function_count as f64
        );
        println!(
            "  Average AIRD Score: {:.2}",
            total_aird as f64 / function_count as f64
        );
        println!(
            "  Average AICP Score: {:.2}",
            total_aicp as f64 / function_count as f64
        );
    }

    Ok(())
}

pub(crate) fn sarif_level(complexity: u32) -> &'static str {
    match complexity {
        0..=10 => "none",
        11..=20 => "note",
        21..=49 => "warning",
        _ => "error",
    }
}

/// Convert a filesystem path string to a relative URI suitable for SARIF
/// artifactLocation. Uses a path relative to the current working directory
/// when possible, otherwise falls back to the raw path.
pub(crate) fn path_to_sarif_uri(file_path: &str) -> String {
    let path = Path::new(file_path);
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    file_path.replace('\\', "/")
}

/// Emit a SARIF 2.1.0 log to stdout describing functions whose max(McCabe, Cognitive)
/// exceeds the healthy threshold (>10). One result per offending function.
pub(crate) fn emit_sarif(all_metrics: &[FunctionMetrics]) -> Result<()> {
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

pub(crate) fn emit_json(all_metrics: &[FunctionMetrics]) -> Result<()> {
    let records: Vec<_> = all_metrics
        .iter()
        .map(|f| {
            json!({
                "file": f.file_path,
                "function": f.name,
                "start_line": f.start_line,
                "end_line": f.end_line,
                "mccabe": f.mccabe,
                "cognitive": f.cognitive,
                "nesting": f.nesting,
                "sloc": f.sloc,
                "abc_magnitude": f.abc_magnitude,
                "return_count": f.return_count,
                "test_score": f.test_scoring.total_score,
                "doc_score": f.test_scoring.documentation_score,
                "aird": f.aird,
                "aicp": f.aicp,
                "external_calls": f.external_calls
            })
        })
        .collect();

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &records).context("Failed to write JSON")?;
    writeln!(handle)?;
    Ok(())
}

pub(crate) fn emit_ndjson(all_metrics: &[FunctionMetrics]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for f in all_metrics {
        let record = json!({
            "file": f.file_path,
            "function": f.name,
            "start_line": f.start_line,
            "end_line": f.end_line,
            "mccabe": f.mccabe,
            "cognitive": f.cognitive,
            "nesting": f.nesting,
            "sloc": f.sloc,
            "abc_magnitude": f.abc_magnitude,
            "return_count": f.return_count,
            "test_score": f.test_scoring.total_score,
            "doc_score": f.test_scoring.documentation_score,
            "aird": f.aird,
            "aicp": f.aicp,
            "external_calls": f.external_calls
        });
        serde_json::to_writer(&mut handle, &record).context("Failed to write NDJSON")?;
        writeln!(handle)?;
    }
    Ok(())
}

pub(crate) fn emit_csv(all_metrics: &[FunctionMetrics]) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    writeln!(
        handle,
        "file,function,start_line,end_line,mccabe,cognitive,nesting,sloc,abc_magnitude,return_count,test_score,doc_score,aird,aicp,external_calls"
    )?;

    for f in all_metrics {
        let name = if f.name.contains(',') {
            format!("\"{}\"", f.name.replace('"', "\"\""))
        } else {
            f.name.clone()
        };
        writeln!(
            handle,
            "{},{},{},{},{},{},{},{},{:.4},{},{},{},{},{},{}",
            f.file_path,
            name,
            f.start_line,
            f.end_line,
            f.mccabe,
            f.cognitive,
            f.nesting,
            f.sloc,
            f.abc_magnitude,
            f.return_count,
            f.test_scoring.total_score,
            f.test_scoring.documentation_score,
            f.aird,
            f.aicp,
            f.external_calls
        )?;
    }
    Ok(())
}

pub(crate) fn write_detailed_report(
    all_metrics: &[FunctionMetrics],
    verbose: bool,
    path: &Path,
) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create report file: {}", path.display()))?;

    for func in all_metrics {
        let emoji = get_complexity_emoji(func.max_complexity());

        if verbose {
            writeln!(file, "Function: {} {}", func_location(func), emoji)?;
            writeln!(file, "  McCabe Complexity: {}", func.mccabe)?;
            writeln!(file, "  Cognitive Complexity: {}", func.cognitive)?;
            writeln!(file, "  Nesting Depth: {}", func.nesting)?;
            writeln!(file, "  SLOC: {}", func.sloc)?;
            writeln!(file, "  ABC Magnitude: {:.2}", func.abc_magnitude)?;
            writeln!(file, "  Return Count: {}", func.return_count)?;
            writeln!(
                file,
                "  Test Scoring: {} ({})",
                func.test_scoring.total_score,
                func.test_scoring.classification()
            )?;
            writeln!(
                file,
                "    - Signature: {}",
                func.test_scoring.signature_score
            )?;
            writeln!(
                file,
                "    - Dependency: {}",
                func.test_scoring.dependency_score
            )?;
            writeln!(
                file,
                "    - Observable: {}",
                func.test_scoring.observable_score
            )?;
            writeln!(
                file,
                "    - Implementation: {}",
                func.test_scoring.implementation_score
            )?;
            writeln!(
                file,
                "    - Documentation: {}",
                func.test_scoring.documentation_score
            )?;
            writeln!(file, "  AIRD Score: {}", func.aird)?;
            writeln!(file, "  AICP Score: {}", func.aicp)?;
            writeln!(file, "  External Calls: {}", func.external_calls)?;
            writeln!(file, "  Max Complexity: {}", func.max_complexity())?;
            writeln!(file)?;
        } else {
            writeln!(
                file,
                "{} {} (McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {}, AIRD: {}, AICP: {}, ExtCalls: {})",
                emoji, func_location(func), func.mccabe, func.cognitive, func.nesting,
                func.sloc, func.abc_magnitude, func.return_count, func.test_scoring.total_score,
                func.aird, func.aicp, func.external_calls
            )?;
        }
    }

    Ok(())
}

pub(crate) fn display_recursive_summary(
    all_metrics: &[FunctionMetrics],
    total_files: usize,
    skipped_files: usize,
    report_path: Option<&Path>,
) {
    let mut sorted = all_metrics.to_vec();
    sorted.sort_by_key(|b| Reverse(b.max_complexity()));

    println!("\n=== TOP 5 WORST FUNCTIONS ===\n");
    for (i, func) in sorted.iter().take(5).enumerate() {
        let emoji = get_complexity_emoji(func.max_complexity());
        println!("{}. {} {}", i + 1, emoji, func_location(func));
        println!("   McCabe: {}, Cognitive: {}, Nesting: {}, SLOC: {}, ABC: {:.2}, Returns: {}, TestScore: {}, AIRD: {}, AICP: {}, ExtCalls: {}",
            func.mccabe, func.cognitive, func.nesting, func.sloc, func.abc_magnitude, func.return_count, func.test_scoring.total_score, func.aird, func.aicp, func.external_calls
        );
    }

    let mut total_mccabe: u64 = 0;
    let mut total_cognitive: u64 = 0;
    let mut total_nesting: u64 = 0;
    let mut total_sloc: u64 = 0;
    let mut total_abc_magnitude = 0.0;
    let mut total_return_count: u64 = 0;
    let mut total_test_score: i64 = 0;
    let mut total_aird: u64 = 0;
    let mut total_aicp: u64 = 0;

    for func in all_metrics {
        total_mccabe += func.mccabe as u64;
        total_cognitive += func.cognitive as u64;
        total_nesting += func.nesting as u64;
        total_sloc += func.sloc as u64;
        total_abc_magnitude += func.abc_magnitude;
        total_return_count += func.return_count as u64;
        total_test_score += func.test_scoring.total_score as i64;
        total_aird += func.aird as u64;
        total_aicp += func.aicp as u64;
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
    println!("  Total AIRD Score: {}", total_aird);
    println!("  Total AICP Score: {}", total_aicp);

    if function_count > 0 {
        println!();
        println!(
            "  Average McCabe Complexity: {:.2}",
            total_mccabe as f64 / function_count as f64
        );
        println!(
            "  Average Cognitive Complexity: {:.2}",
            total_cognitive as f64 / function_count as f64
        );
        println!(
            "  Average Nesting Depth: {:.2}",
            total_nesting as f64 / function_count as f64
        );
        println!(
            "  Average SLOC: {:.2}",
            total_sloc as f64 / function_count as f64
        );
        println!(
            "  Average ABC Magnitude: {:.2}",
            total_abc_magnitude / function_count as f64
        );
        println!(
            "  Average Return Count: {:.2}",
            total_return_count as f64 / function_count as f64
        );
        println!(
            "  Average Test Score: {:.2}",
            total_test_score as f64 / function_count as f64
        );
        println!(
            "  Average AIRD Score: {:.2}",
            total_aird as f64 / function_count as f64
        );
        println!(
            "  Average AICP Score: {:.2}",
            total_aicp as f64 / function_count as f64
        );
    }

    if let Some(path) = report_path {
        println!(
            "\nDetailed per-function output written to {}",
            path.display()
        );
    }
    println!("\n=== FILES PROCESSED ===\n");
    println!("  Total files found: {}", total_files);
    println!("  Successfully processed: {}", total_files - skipped_files);
    if skipped_files > 0 {
        println!("  Skipped (encoding/parse errors): {}", skipped_files);
    }
}

pub(crate) fn display_testability_matrix(
    all_metrics: &[FunctionMetrics],
    total_files: usize,
    skipped_files: usize,
) {
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

    println!("\n=== TESTABILITY MATRIX ===\n");

    println!("📊 QUICK WINS (Low Complexity, Easy to Test) - Automate!");
    println!("=========================================================");
    if quick_wins.is_empty() {
        println!("  (none)");
    } else {
        for func in &quick_wins {
            if func.file_path.is_empty() {
                println!(
                    "  ✓ {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  ✓ {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
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
                println!(
                    "  → {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  → {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
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
                println!(
                    "  ⚠ {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  ⚠ {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
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
                println!(
                    "  ⛔ {} (McCabe: {}, TestScore: {})",
                    func.name, func.mccabe, func.test_scoring.total_score
                );
            } else {
                println!(
                    "  ⛔ {} [{}] (McCabe: {}, TestScore: {})",
                    func.name, func.file_path, func.mccabe, func.test_scoring.total_score
                );
            }
        }
    }
    println!();

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
