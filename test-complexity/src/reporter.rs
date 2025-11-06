use colored::*;
use crate::analyzer::AnalysisResult;
use std::path::Path;

pub struct Reporter {
    verbose: bool,
}

impl Reporter {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn print_report(&self, result: &AnalysisResult) {
        // Extract base filenames for cleaner display
        let test_name = Path::new(&result.test_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&result.test_file);
        let source_name = Path::new(&result.source_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&result.source_file);

        println!("\n{}", "━".repeat(70).bright_black());
        println!("{}", format!("Test Quality Analysis: {}", test_name).bold());
        println!("{}\n", "━".repeat(70).bright_black());

        // Source metrics
        println!("{}", "Source File:".bold());
        println!("  File: {}", source_name);
        println!("  Functions: {}", result.source_function_count);
        println!("  Total Cyclomatic Complexity: {}", result.source_cyclomatic_complexity);
        println!("  Total Cognitive Complexity: {}", result.source_cognitive_complexity);

        // Test metrics
        println!("\n{}", "Test File:".bold());
        println!("  File: {}", test_name);
        println!("  Functions: {}", result.test_function_count);
        println!("  Total Cyclomatic Complexity: {}", result.test_cyclomatic_complexity);
        println!("  Total Cognitive Complexity: {}", result.test_cognitive_complexity);

        // Ratio analysis
        println!("\n{}", "Complexity Analysis:".bold());
        let cyclomatic_percent = (result.cyclomatic_ratio * 100.0) as i32;
        let threshold_percent = (result.threshold * 100.0) as i32;

        let status = if result.passed {
            format!("{}% ✓", cyclomatic_percent).green()
        } else {
            format!("{}% ✗", cyclomatic_percent).red()
        };

        println!("  Test/Source Ratio: {} (threshold: {}%)", status, threshold_percent);
        println!("  Test Cyclomatic Complexity: {}", result.test_cyclomatic_complexity);
        println!("  Source Cyclomatic Complexity: {}", result.source_cyclomatic_complexity);

        if self.verbose {
            println!("\n  Cognitive Complexity (informational):");
            println!("    Test: {}", result.test_cognitive_complexity);
            println!("    Source: {}", result.source_cognitive_complexity);
            println!("    Ratio: {:.0}%", result.cognitive_ratio * 100.0);
        }

        // Boundary analysis
        if let Some(boundary) = &result.boundary_analysis {
            println!("\n{}", "Boundary Analysis:".bold());
            let boundary_count = boundary.required_boundaries.len();

            if boundary_count > 0 {
                println!("  Boundary Values Detected: {}", boundary_count);

                let boundary_threshold_percent = (result.boundary_threshold * 100.0) as i32;
                let coverage_status = if boundary.coverage_percent >= (result.boundary_threshold * 100.0) {
                    format!("{:.0}% ✓", boundary.coverage_percent).green()
                } else {
                    format!("{:.0}% ✗", boundary.coverage_percent).red()
                };

                println!("  Boundary Test Coverage: {} (threshold: {}%)", coverage_status, boundary_threshold_percent);
                println!("  Test Values Found: {}", boundary.found_test_values.len());

                // Show sample boundary values detected
                if self.verbose && !boundary.required_boundaries.is_empty() {
                    println!("\n  Detected Boundaries:");
                    for (i, bv) in boundary.required_boundaries.iter().take(5).enumerate() {
                        println!("    {}. {} ({}) - range: {} to {}",
                            i + 1,
                            bv.variable_name,
                            bv.type_name,
                            bv.min_value,
                            bv.max_value
                        );
                    }
                    if boundary.required_boundaries.len() > 5 {
                        println!("    ... and {} more", boundary.required_boundaries.len() - 5);
                    }
                }
            } else {
                println!("  No boundary values detected in source (no integer type variables)");
            }
        }

        // Recommendations
        if !result.recommendations.is_empty() {
            println!("\n{}", "Recommendations:".bold().yellow());
            for rec in &result.recommendations {
                println!("{}", rec.yellow());
            }
        }

        // Final result
        println!("\n{}", "━".repeat(70).bright_black());
        if result.passed {
            println!("{}", "Result: ✓ PASS".green().bold());
        } else {
            println!("{}", "Result: ✗ FAIL".red().bold());
        }
        println!("{}\n", "━".repeat(70).bright_black());
    }
}
