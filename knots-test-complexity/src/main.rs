use anyhow::Result;
use clap::Parser;

mod analyzer;
mod boundary;
mod reporter;

use analyzer::TestQualityAnalyzer;
use reporter::Reporter;

#[derive(Parser)]
#[command(name = "knots-test-complexity")]
#[command(version)]
#[command(about = "Test quality analyzer for C unit tests - validates test complexity against source complexity", long_about = None)]
struct Args {
    /// Test file path (e.g., Test/test_battery_service.c)
    test_file: String,

    /// Source file path (e.g., Core/Src/modules/battery_service/battery_service.c)
    source_file: String,

    /// Minimum test-to-source complexity ratio (default: 0.70 = 70%)
    #[arg(short, long, default_value = "0.70")]
    threshold: f64,

    /// Minimum boundary test coverage ratio (default: 0.80 = 80%)
    #[arg(short = 'b', long, default_value = "0.80")]
    boundary_threshold: f64,

    /// Enforcement level: warn or error
    #[arg(short, long, default_value = "warn")]
    level: String,

    /// Disable boundary value checking (boundary checking is enabled by default)
    #[arg(long)]
    no_check_boundaries: bool,

    /// Verbose output (shows detailed per-function analysis)
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate inputs
    if args.threshold < 0.0 || args.threshold > 2.0 {
        eprintln!("Error: threshold must be between 0.0 and 2.0");
        std::process::exit(1);
    }

    if args.boundary_threshold < 0.0 || args.boundary_threshold > 1.0 {
        eprintln!("Error: boundary-threshold must be between 0.0 and 1.0");
        std::process::exit(1);
    }

    if args.level != "warn" && args.level != "error" {
        eprintln!("Error: level must be 'warn' or 'error'");
        std::process::exit(1);
    }

    // Check if files exist
    if !std::path::Path::new(&args.test_file).exists() {
        eprintln!("Error: Test file not found: {}", args.test_file);
        std::process::exit(1);
    }

    if !std::path::Path::new(&args.source_file).exists() {
        eprintln!("Error: Source file not found: {}", args.source_file);
        std::process::exit(1);
    }

    // Create analyzer and run analysis
    let analyzer = TestQualityAnalyzer::new(
        &args.test_file,
        &args.source_file,
        args.threshold,
        args.boundary_threshold,
    )?;

    let result = analyzer.analyze(!args.no_check_boundaries);

    // Generate report
    let reporter = Reporter::new(args.verbose);
    reporter.print_report(&result);

    // Exit based on enforcement level and result
    if !result.passed && args.level == "error" {
        std::process::exit(1);
    }

    Ok(())
}
