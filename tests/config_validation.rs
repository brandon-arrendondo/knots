//! A `knots.toml` section or key that knots does not read stops the run with
//! the file, line and nearest valid name, instead of leaving a gate off.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/config")
        .join(name)
}

fn run(name: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knots"))
        .current_dir(fixture(name))
        .args(["a.c", "--format", "ndjson"])
        .output()
        .unwrap()
}

#[test]
fn unknown_language_section_is_an_error() {
    let out = run("unknown-language");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("knots.toml:5: unknown section `c++`; did you mean `cpp`?"),
        "{stderr}"
    );
    assert!(out.stdout.is_empty(), "nothing is analysed");
}

#[test]
fn misspelled_threshold_key_is_an_error() {
    let out = run("misspelled-key");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains(
            "knots.toml:3: unknown key `mccab` in `[c.thresholds]`; did you mean `mccabe`?"
        ),
        "{stderr}"
    );
}

#[test]
fn valid_config_loads_without_complaint() {
    let out = run("valid");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The per-language McCabe threshold of 1 applies, so `f` (McCabe 2) fails
    // the gate: the config was read, not merely accepted.
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("McCabe 2 > 1"), "{stderr}");
    assert!(!stderr.contains("unknown"), "{stderr}");
}
