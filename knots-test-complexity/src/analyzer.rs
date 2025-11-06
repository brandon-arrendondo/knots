use anyhow::Result;
use tree_sitter::{Node, Parser};
use crate::boundary::{BoundaryAnalysis, BoundaryDetector};
use knots::{calculate_mccabe_complexity, calculate_cognitive_complexity};

#[derive(Debug, Clone)]
pub struct FunctionMetrics {
    pub function_name: String,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub line_start: usize,
    pub line_end: usize,
}

pub struct FileAnalysis {
    pub file_path: String,
    pub functions: Vec<FunctionMetrics>,
    pub total_cyclomatic_complexity: u32,
    pub total_cognitive_complexity: u32,
}

impl FileAnalysis {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            functions: Vec::new(),
            total_cyclomatic_complexity: 0,
            total_cognitive_complexity: 0,
        }
    }

    pub fn add_function(&mut self, metrics: FunctionMetrics) {
        self.total_cyclomatic_complexity += metrics.cyclomatic_complexity;
        self.total_cognitive_complexity += metrics.cognitive_complexity;
        self.functions.push(metrics);
    }
}

pub struct TestQualityAnalyzer {
    pub test_analysis: FileAnalysis,
    pub source_analysis: FileAnalysis,
    pub threshold: f64,
    pub boundary_threshold: f64,
}

pub struct AnalysisResult {
    pub passed: bool,
    pub test_cyclomatic_complexity: u32,
    pub source_cyclomatic_complexity: u32,
    pub test_cognitive_complexity: u32,
    pub source_cognitive_complexity: u32,
    pub cyclomatic_ratio: f64,
    pub cognitive_ratio: f64,
    pub threshold: f64,
    pub boundary_threshold: f64,
    pub test_function_count: usize,
    pub source_function_count: usize,
    pub recommendations: Vec<String>,
    pub test_file: String,
    pub source_file: String,
    pub boundary_analysis: Option<BoundaryAnalysis>,
}

impl TestQualityAnalyzer {
    pub fn new(
        test_file: &str,
        source_file: &str,
        threshold: f64,
        boundary_threshold: f64,
    ) -> Result<Self> {
        let test_analysis = analyze_file(test_file)?;
        let source_analysis = analyze_file(source_file)?;

        Ok(Self {
            test_analysis,
            source_analysis,
            threshold,
            boundary_threshold,
        })
    }

    pub fn analyze(&self, check_boundaries: bool) -> AnalysisResult {
        let test_cyclomatic = self.test_analysis.total_cyclomatic_complexity;
        let source_cyclomatic = self.source_analysis.total_cyclomatic_complexity;
        let test_cognitive = self.test_analysis.total_cognitive_complexity;
        let source_cognitive = self.source_analysis.total_cognitive_complexity;

        // Calculate cyclomatic ratio
        let cyclomatic_ratio = if source_cyclomatic > 0 {
            test_cyclomatic as f64 / source_cyclomatic as f64
        } else {
            1.0 // No source complexity = trivial, always pass
        };

        // Calculate cognitive ratio (for reporting and future ceiling checks)
        let cognitive_ratio = if source_cognitive > 0 {
            test_cognitive as f64 / source_cognitive as f64
        } else {
            1.0
        };

        // Use cyclomatic ratio only for pass/fail determination
        // Cognitive complexity is tracked but not used in threshold calculation
        let mut passed = cyclomatic_ratio >= self.threshold;

        // Perform boundary analysis if requested
        let boundary_analysis = if check_boundaries {
            match self.analyze_boundaries() {
                Ok(analysis) => {
                    // Boundary coverage below threshold is a failure
                    if analysis.coverage_percent < (self.boundary_threshold * 100.0) {
                        passed = false;
                    }
                    Some(analysis)
                }
                Err(e) => {
                    eprintln!("Warning: Boundary analysis failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let mut recommendations = Vec::new();
        if !passed {
            self.generate_recommendations(&mut recommendations, cyclomatic_ratio, &boundary_analysis);
        }

        AnalysisResult {
            passed,
            test_cyclomatic_complexity: test_cyclomatic,
            source_cyclomatic_complexity: source_cyclomatic,
            test_cognitive_complexity: test_cognitive,
            source_cognitive_complexity: source_cognitive,
            cyclomatic_ratio,
            cognitive_ratio,
            threshold: self.threshold,
            boundary_threshold: self.boundary_threshold,
            test_function_count: self.test_analysis.functions.len(),
            source_function_count: self.source_analysis.functions.len(),
            recommendations,
            test_file: self.test_analysis.file_path.clone(),
            source_file: self.source_analysis.file_path.clone(),
            boundary_analysis,
        }
    }

    fn analyze_boundaries(&self) -> Result<BoundaryAnalysis> {
        let mut detector = BoundaryDetector::new();
        detector.detect_boundaries(&self.source_analysis.file_path)?;
        detector.analyze_test_coverage(&self.test_analysis.file_path)
    }

    fn generate_recommendations(&self, recommendations: &mut Vec<String>, cyclomatic_ratio: f64, boundary_analysis: &Option<BoundaryAnalysis>) {
        // Only generate complexity recommendations if complexity ratio failed
        if cyclomatic_ratio < self.threshold {
            let gap_percent = ((self.threshold - cyclomatic_ratio) * 100.0) as i32;

            // Use average of both target complexities
            let target_cyclomatic = (self.source_analysis.total_cyclomatic_complexity as f64 * self.threshold) as u32;
            let target_cognitive = (self.source_analysis.total_cognitive_complexity as f64 * self.threshold) as u32;
            let missing_cyclomatic = target_cyclomatic.saturating_sub(self.test_analysis.total_cyclomatic_complexity);
            let missing_cognitive = target_cognitive.saturating_sub(self.test_analysis.total_cognitive_complexity);
            let avg_missing = (missing_cyclomatic + missing_cognitive) / 2;

            recommendations.push(format!(
                "Add ~{} more complexity points to tests ({} percentage points below threshold)",
                avg_missing, gap_percent
            ));

            recommendations.push("Consider adding:".to_string());
            recommendations.push("  - Edge case tests (boundary values, overflow scenarios)".to_string());
            recommendations.push("  - Error path tests (invalid inputs, error conditions)".to_string());
            recommendations.push("  - State transition tests (different initial conditions)".to_string());
            recommendations.push("  - Parametrized tests or loops in test code".to_string());
        }

        // Identify high-complexity source functions that might need more testing
        let mut high_complexity_funcs: Vec<_> = self.source_analysis.functions.iter()
            .filter(|f| f.cyclomatic_complexity > 5)
            .collect();
        high_complexity_funcs.sort_by_key(|f| std::cmp::Reverse(f.cyclomatic_complexity));

        if !high_complexity_funcs.is_empty() {
            recommendations.push("\nComplex functions needing thorough tests:".to_string());
            for func in high_complexity_funcs.iter().take(5) {
                recommendations.push(format!(
                    "  - {}() [complexity: {}] at lines {}-{}",
                    func.function_name,
                    func.cyclomatic_complexity,
                    func.line_start,
                    func.line_end
                ));
            }
        }

        // Add boundary-specific recommendations
        if let Some(boundary) = boundary_analysis {
            if boundary.coverage_percent < 80.0 && !boundary.missing_boundaries.is_empty() {
                recommendations.push("\nMissing boundary value tests:".to_string());
                for (i, missing) in boundary.missing_boundaries.iter().take(5).enumerate() {
                    recommendations.push(format!("  {}. {}", i + 1, missing));
                }
                if boundary.missing_boundaries.len() > 5 {
                    recommendations.push(format!("  ... and {} more", boundary.missing_boundaries.len() - 5));
                }
            }
        }
    }
}

/// Analyze a C file and extract function complexity metrics using knots
pub fn analyze_file(file_path: &str) -> Result<FileAnalysis> {
    let source_code = std::fs::read(file_path)?;

    let mut parser = Parser::new();
    let language = tree_sitter_c::language();
    parser.set_language(&language)?;

    let tree = parser.parse(&source_code, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse file: {}", file_path))?;

    let root_node = tree.root_node();
    let mut file_analysis = FileAnalysis::new(file_path.to_string());

    // Find all function definitions
    visit_functions(&root_node, &source_code, &mut |node| {
        let metrics = extract_function_metrics(&node, &source_code);
        file_analysis.add_function(metrics);
    });

    Ok(file_analysis)
}

fn visit_functions<F>(node: &Node, source_code: &[u8], callback: &mut F)
where
    F: FnMut(Node),
{
    if node.kind() == "function_definition" {
        callback(*node);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_functions(&child, source_code, callback);
    }
}

fn extract_function_metrics(node: &Node, source: &[u8]) -> FunctionMetrics {
    let function_name = extract_function_name(node, source);

    // Use knots' complexity calculations directly
    let cyclomatic_complexity = calculate_mccabe_complexity(*node, source);
    let cognitive_complexity = calculate_cognitive_complexity(*node, source);

    let line_start = node.start_position().row + 1;
    let line_end = node.end_position().row + 1;

    FunctionMetrics {
        function_name,
        cyclomatic_complexity,
        cognitive_complexity,
        line_start,
        line_end,
    }
}

fn extract_function_name(node: &Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            return get_declarator_name(&child, source);
        } else if child.kind() == "pointer_declarator" {
            // For functions returning pointers
            if let Some(name) = get_function_name_from_declarator(&child, source) {
                return name;
            }
        }
    }
    "unknown".to_string()
}

fn get_function_name_from_declarator(node: &Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "function_declarator" {
            return Some(get_declarator_name(&child, source));
        } else if child.kind() == "pointer_declarator" {
            if let Some(name) = get_function_name_from_declarator(&child, source) {
                return Some(name);
            }
        }
    }

    None
}

fn get_declarator_name(node: &Node, source: &[u8]) -> String {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            if let Ok(text) = child.utf8_text(source) {
                return text.to_string();
            }
        } else if child.kind() == "pointer_declarator" || child.kind() == "function_declarator" {
            let name = get_declarator_name(&child, source);
            if name != "unknown" {
                return name;
            }
        }
    }

    "unknown".to_string()
}
