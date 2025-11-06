use anyhow::Result;
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct BoundaryValue {
    pub variable_name: String,
    pub type_name: String,
    pub min_value: i64,
    pub max_value: i64,
}

impl BoundaryValue {
    pub fn boundary_values(&self) -> Vec<i64> {
        vec![
            self.min_value,
            self.min_value.saturating_sub(1),
            self.max_value,
            self.max_value.saturating_add(1),
        ]
    }
}

pub struct BoundaryDetector {
    boundaries: Vec<BoundaryValue>,
}

pub struct BoundaryAnalysis {
    pub required_boundaries: Vec<BoundaryValue>,
    pub found_test_values: HashSet<i64>,
    pub coverage_percent: f64,
    pub missing_boundaries: Vec<String>,
}

impl BoundaryDetector {
    pub fn new() -> Self {
        Self {
            boundaries: Vec::new(),
        }
    }

    /// Detect boundary values in source code
    pub fn detect_boundaries(&mut self, file_path: &str) -> Result<Vec<BoundaryValue>> {
        let source_code = std::fs::read_to_string(file_path)?;

        // Detect integer type declarations
        self.detect_integer_types(&source_code)?;

        // Detect range checks and constants
        self.detect_range_checks(&source_code)?;

        Ok(self.boundaries.clone())
    }

    /// Detect integer type declarations (uint8_t, uint16_t, etc.)
    fn detect_integer_types(&mut self, source: &str) -> Result<()> {
        let type_patterns = vec![
            ("uint8_t", 0, 255),
            ("uint16_t", 0, 65535),
            ("uint32_t", 0, 4294967295i64),
            ("int8_t", -128, 127),
            ("int16_t", -32768, 32767),
            ("int32_t", -2147483648i64, 2147483647i64),
        ];

        for (type_name, min_val, max_val) in type_patterns {
            // Regex to find variable declarations
            // Matches: uint8_t foo; or uint8_t foo = 0; or uint8_t foo, bar;
            let pattern = format!(r"\b{}\s+(\w+)\s*[;=,]", type_name);
            let re = Regex::new(&pattern)?;

            for captures in re.captures_iter(source) {
                if let Some(var_name) = captures.get(1) {
                    let var_str = var_name.as_str();

                    // Skip common prefixes that might not be actual variables
                    if var_str.starts_with("MAX_") || var_str.starts_with("MIN_") {
                        continue;
                    }

                    self.boundaries.push(BoundaryValue {
                        variable_name: var_str.to_string(),
                        type_name: type_name.to_string(),
                        min_value: min_val,
                        max_value: max_val,
                    });
                }
            }
        }

        Ok(())
    }

    /// Detect range checks (if (x > MAX), if (x < MIN), etc.)
    fn detect_range_checks(&mut self, source: &str) -> Result<()> {
        // Patterns to detect comparison with constants
        let patterns = vec![
            // if (x > CONSTANT) or if (x >= CONSTANT)
            (r"if\s*\(\s*\w+\s*>=?\s*(\d+)", "range_check_upper"),
            // if (x < CONSTANT) or if (x <= CONSTANT)
            (r"if\s*\(\s*\w+\s*<=?\s*(\d+)", "range_check_lower"),
            // if (CONSTANT < x) or if (CONSTANT <= x)
            (r"if\s*\(\s*(\d+)\s*<=?\s*\w+", "range_check_lower"),
            // if (CONSTANT > x) or if (CONSTANT >= x)
            (r"if\s*\(\s*(\d+)\s*>=?\s*\w+", "range_check_upper"),
            // Defined constants like #define MAX_VALUE 255
            (r"#define\s+\w*MAX\w*\s+(\d+)", "constant_max"),
            (r"#define\s+\w*MIN\w*\s+(\d+)", "constant_min"),
        ];

        for (pattern_str, boundary_type) in patterns {
            let re = Regex::new(pattern_str)?;

            for captures in re.captures_iter(source) {
                if let Some(value_match) = captures.get(1) {
                    if let Ok(value) = value_match.as_str().parse::<i64>() {
                        // Create boundary based on the constant
                        let (min_val, max_val) = if boundary_type.contains("upper") || boundary_type.contains("max") {
                            // Upper bound: test value and value+1
                            (value.saturating_sub(1), value)
                        } else {
                            // Lower bound: test value-1 and value
                            (value, value.saturating_add(1))
                        };

                        self.boundaries.push(BoundaryValue {
                            variable_name: format!("constant_{}", value),
                            type_name: boundary_type.to_string(),
                            min_value: min_val,
                            max_value: max_val,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Count boundary tests in test file
    pub fn analyze_test_coverage(&self, test_file_path: &str) -> Result<BoundaryAnalysis> {
        let source_code = std::fs::read_to_string(test_file_path)?;
        let mut found_values = HashSet::new();

        // Extract all numeric literals from test file (including negative numbers)
        let number_re = Regex::new(r"(-?\d+)\b")?;

        for captures in number_re.captures_iter(&source_code) {
            if let Some(num_match) = captures.get(1) {
                if let Ok(value) = num_match.as_str().parse::<i64>() {
                    found_values.insert(value);
                }
            }
        }

        // Also look for hex literals (0xFF, 0xFFFF, etc.)
        let hex_re = Regex::new(r"\b(0[xX][0-9a-fA-F]+)\b")?;
        for captures in hex_re.captures_iter(&source_code) {
            if let Some(hex_match) = captures.get(1) {
                let hex_str = hex_match.as_str();
                if let Ok(value) = i64::from_str_radix(&hex_str[2..], 16) {
                    found_values.insert(value);
                }
            }
        }

        // Calculate coverage
        let mut total_required = 0;
        let mut total_found = 0;
        let mut missing = Vec::new();

        for boundary in &self.boundaries {
            let boundary_vals = boundary.boundary_values();
            let required_count = boundary_vals.len();
            let found_count = boundary_vals.iter()
                .filter(|v| found_values.contains(v))
                .count();

            total_required += required_count;
            total_found += found_count;

            // Track missing boundaries
            if found_count < required_count {
                let missing_vals: Vec<String> = boundary_vals.iter()
                    .filter(|v| !found_values.contains(v))
                    .map(|v| v.to_string())
                    .collect();

                missing.push(format!(
                    "{} ({}): missing values [{}]",
                    boundary.variable_name,
                    boundary.type_name,
                    missing_vals.join(", ")
                ));
            }
        }

        let coverage_percent = if total_required > 0 {
            (total_found as f64 / total_required as f64) * 100.0
        } else {
            100.0 // No boundaries required = 100% coverage
        };

        Ok(BoundaryAnalysis {
            required_boundaries: self.boundaries.clone(),
            found_test_values: found_values,
            coverage_percent,
            missing_boundaries: missing,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_uint8_boundary() {
        let code = r#"
        uint8_t counter = 0;
        uint16_t timer_ms = 0;
        "#;

        let mut detector = BoundaryDetector::new();
        detector.detect_integer_types(code).unwrap();

        assert_eq!(detector.boundaries.len(), 2);
        assert_eq!(detector.boundaries[0].type_name, "uint8_t");
        assert_eq!(detector.boundaries[0].min_value, 0);
        assert_eq!(detector.boundaries[0].max_value, 255);
        assert_eq!(detector.boundaries[1].type_name, "uint16_t");
        assert_eq!(detector.boundaries[1].max_value, 65535);
    }

    #[test]
    fn test_detect_range_checks() {
        let code = r#"
        if (counter > 100) {
            // overflow check
        }
        #define MAX_VALUE 255
        "#;

        let mut detector = BoundaryDetector::new();
        detector.detect_range_checks(code).unwrap();

        assert!(detector.boundaries.len() >= 2);
    }
}
