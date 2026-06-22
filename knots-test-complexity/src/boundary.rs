use anyhow::Result;
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct BoundaryValue {
    pub variable_name: String,
    pub type_name: String,
    pub min_value: i64,
    pub max_value: i64,
    pub is_signed: bool,
}

impl BoundaryValue {
    pub fn boundary_values(&self) -> Vec<i64> {
        if self.is_signed {
            vec![
                self.min_value.saturating_sub(1),
                self.min_value,
                self.max_value,
                self.max_value.saturating_add(1),
            ]
        } else {
            // For unsigned types, 0 and TYPE_MAX are the valid boundaries;
            // out-of-range values (-1, TYPE_MAX+1) cannot be held by the type.
            vec![self.min_value, self.max_value]
        }
    }
}

#[derive(Debug, Clone)]
pub struct PointerBoundary {
    pub variable_name: String,
    pub type_name: String,
}

pub struct BoundaryDetector {
    boundaries: Vec<BoundaryValue>,
    pointer_boundaries: Vec<PointerBoundary>,
}

pub struct BoundaryAnalysis {
    pub required_boundaries: Vec<BoundaryValue>,
    pub pointer_boundaries: Vec<PointerBoundary>,
    pub found_test_values: HashSet<i64>,
    pub null_tested: bool,
    pub empty_string_tested: bool,
    pub coverage_percent: f64,
    pub missing_boundaries: Vec<String>,
}

impl BoundaryDetector {
    pub fn new() -> Self {
        Self {
            boundaries: Vec::new(),
            pointer_boundaries: Vec::new(),
        }
    }

    /// Detect boundary values in source code
    pub fn detect_boundaries(&mut self, file_path: &str) -> Result<Vec<BoundaryValue>> {
        let source_code = std::fs::read_to_string(file_path)?;

        self.detect_integer_types(&source_code)?;
        self.detect_range_checks(&source_code)?;
        self.detect_pointer_types(&source_code)?;

        Ok(self.boundaries.clone())
    }

    /// Detect integer type declarations (uint8_t, uint16_t, etc.)
    fn detect_integer_types(&mut self, source: &str) -> Result<()> {
        let type_patterns: Vec<(&str, i64, i64, bool)> = vec![
            ("uint8_t", 0, 255, false),
            ("uint16_t", 0, 65535, false),
            ("uint32_t", 0, 4294967295i64, false),
            ("int8_t", -128, 127, true),
            ("int16_t", -32768, 32767, true),
            ("int32_t", -2147483648i64, 2147483647i64, true),
        ];

        for (type_name, min_val, max_val, is_signed) in type_patterns {
            let pattern = format!(r"\b{}\s+(\w+)\s*[;=,]", type_name);
            let re = Regex::new(&pattern)?;

            for captures in re.captures_iter(source) {
                if let Some(var_name) = captures.get(1) {
                    let var_str = var_name.as_str();

                    if var_str.starts_with("MAX_") || var_str.starts_with("MIN_") {
                        continue;
                    }

                    self.boundaries.push(BoundaryValue {
                        variable_name: var_str.to_string(),
                        type_name: type_name.to_string(),
                        min_value: min_val,
                        max_value: max_val,
                        is_signed,
                    });
                }
            }
        }

        Ok(())
    }

    /// Detect char * / const char * variable and parameter declarations.
    /// NULL and empty string are the meaningful boundary sentinels for these types.
    fn detect_pointer_types(&mut self, source: &str) -> Result<()> {
        // Matches: char *foo, char* foo, const char *foo, const char* foo
        let re = Regex::new(r"\bconst\s+char\s*\*+\s*(\w+)|\bchar\s*\*+\s*(\w+)")?;
        let mut seen = std::collections::HashSet::new();

        for captures in re.captures_iter(source) {
            let var_str = captures
                .get(1)
                .or_else(|| captures.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");

            if var_str.is_empty() || !seen.insert(var_str.to_string()) {
                continue;
            }

            let type_name = if captures.get(1).is_some() {
                "const char *"
            } else {
                "char *"
            };

            self.pointer_boundaries.push(PointerBoundary {
                variable_name: var_str.to_string(),
                type_name: type_name.to_string(),
            });
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
                        let (min_val, max_val) =
                            if boundary_type.contains("upper") || boundary_type.contains("max") {
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
                            is_signed: true,
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

        let number_re = Regex::new(r"(-?\d+)\b")?;
        for captures in number_re.captures_iter(&source_code) {
            if let Some(num_match) = captures.get(1) {
                if let Ok(value) = num_match.as_str().parse::<i64>() {
                    found_values.insert(value);
                }
            }
        }

        let hex_re = Regex::new(r"\b(0[xX][0-9a-fA-F]+)\b")?;
        for captures in hex_re.captures_iter(&source_code) {
            if let Some(hex_match) = captures.get(1) {
                let hex_str = hex_match.as_str();
                if let Ok(value) = i64::from_str_radix(&hex_str[2..], 16) {
                    found_values.insert(value);
                }
            }
        }

        // Check pointer sentinels in test file
        let null_tested = Regex::new(r"\bNULL\b")?.is_match(&source_code);
        let empty_string_tested = source_code.contains("\"\"");

        // Integer boundary coverage
        let mut total_required = 0;
        let mut total_found = 0;
        let mut missing = Vec::new();

        for boundary in &self.boundaries {
            let boundary_vals = boundary.boundary_values();
            let required_count = boundary_vals.len();
            let found_count = boundary_vals
                .iter()
                .filter(|v| found_values.contains(v))
                .count();

            total_required += required_count;
            total_found += found_count;

            if found_count < required_count {
                let missing_vals: Vec<String> = boundary_vals
                    .iter()
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

        // Pointer sentinel coverage: each pointer variable requires NULL + empty-string tests
        for pb in &self.pointer_boundaries {
            total_required += 2;
            if null_tested {
                total_found += 1;
            }
            if empty_string_tested {
                total_found += 1;
            }

            let mut missing_sentinels = Vec::new();
            if !null_tested {
                missing_sentinels.push("NULL");
            }
            if !empty_string_tested {
                missing_sentinels.push("\"\"");
            }

            if !missing_sentinels.is_empty() {
                missing.push(format!(
                    "{} ({}): missing sentinels [{}]",
                    pb.variable_name,
                    pb.type_name,
                    missing_sentinels.join(", ")
                ));
            }
        }

        let coverage_percent = if total_required > 0 {
            (total_found as f64 / total_required as f64) * 100.0
        } else {
            100.0
        };

        Ok(BoundaryAnalysis {
            required_boundaries: self.boundaries.clone(),
            pointer_boundaries: self.pointer_boundaries.clone(),
            found_test_values: found_values,
            null_tested,
            empty_string_tested,
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
        assert!(!detector.boundaries[0].is_signed);
        assert_eq!(detector.boundaries[1].type_name, "uint16_t");
        assert_eq!(detector.boundaries[1].max_value, 65535);
        assert!(!detector.boundaries[1].is_signed);
    }

    #[test]
    fn test_unsigned_boundary_values_exclude_out_of_range() {
        let bv = BoundaryValue {
            variable_name: "x".to_string(),
            type_name: "uint8_t".to_string(),
            min_value: 0,
            max_value: 255,
            is_signed: false,
        };
        let vals = bv.boundary_values();
        assert_eq!(vals, vec![0, 255]);
        assert!(!vals.contains(&-1));
        assert!(!vals.contains(&256));
    }

    #[test]
    fn test_signed_boundary_values_include_out_of_range() {
        let bv = BoundaryValue {
            variable_name: "x".to_string(),
            type_name: "int8_t".to_string(),
            min_value: -128,
            max_value: 127,
            is_signed: true,
        };
        let vals = bv.boundary_values();
        assert_eq!(vals, vec![-129, -128, 127, 128]);
    }

    #[test]
    fn test_detect_pointer_types() {
        let code = r#"
        int foo(const char *hostname, char *buf) {
            char *tmp = NULL;
        }
        "#;

        let mut detector = BoundaryDetector::new();
        detector.detect_pointer_types(code).unwrap();

        assert!(!detector.pointer_boundaries.is_empty());
        let names: Vec<&str> = detector
            .pointer_boundaries
            .iter()
            .map(|p| p.variable_name.as_str())
            .collect();
        assert!(names.contains(&"hostname"));
        assert!(names.contains(&"buf"));
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
