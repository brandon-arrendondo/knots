//! What a scan selected but could not analyse, so that an incomplete scan is
//! reported and never passes as clean (docs/adr/0005).
//!
//! Every pass that reads, parses or analyses a selected file reports its
//! failures here instead of dropping the file. [`finish`] then gives an
//! incomplete scan the [`INCOMPLETE_EXIT`] status, whatever the gate found.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::json;
use std::any::Any;
use std::fmt;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

/// Exit status of a scan that left any selected file unanalysed. It outranks
/// a threshold violation (1), and clap keeps 2 for invalid arguments.
pub(crate) const INCOMPLETE_EXIT: u8 = 3;

/// The step at which a selected file was lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Stage {
    /// It could not be read, or is not UTF-8.
    Read,
    /// No grammar could parse it.
    Parse,
    /// Its analysis panicked.
    Crash,
    /// Named on the command line, in a language knots cannot analyse.
    Unsupported,
    /// Listed in compile_commands.json, but not on disk.
    Missing,
}

impl Stage {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Stage::Read => "read",
            Stage::Parse => "parse",
            Stage::Crash => "crash",
            Stage::Unsupported => "unsupported",
            Stage::Missing => "missing",
        }
    }
}

/// One selected file that produced no analysis, and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct LostInput {
    pub(crate) file: String,
    pub(crate) stage: Stage,
    pub(crate) reason: String,
}

impl LostInput {
    pub(crate) fn new(file: &Path, stage: Stage, reason: impl Into<String>) -> Self {
        LostInput {
            file: file.to_string_lossy().into_owned(),
            stage,
            reason: reason.into(),
        }
    }
}

/// What one scan selected, lost and recovered.
#[derive(Debug, Default)]
pub(crate) struct Completion {
    selected: usize,
    lost: Vec<LostInput>,
    recovered_no_functions: Vec<String>,
}

impl Completion {
    /// A scan of `selected` files, of which `lost` were lost before any was
    /// read (named but unsupported, or missing from compile_commands.json).
    pub(crate) fn new(selected: usize, lost: Vec<LostInput>) -> Self {
        let mut completion = Completion {
            selected: selected + lost.len(),
            ..Completion::default()
        };
        for input in lost {
            completion.lose(input);
        }
        completion
    }

    /// Records a lost file, once: a later pass that loses an already-lost
    /// file adds nothing.
    pub(crate) fn lose(&mut self, input: LostInput) {
        if !self.is_lost(&input.file) {
            eprintln!(
                "Warning: Not analysed ({}): {}: {}",
                input.stage.name(),
                input.file,
                input.reason
            );
            self.lost.push(input);
        }
    }

    /// Records a file that parsed only with syntax errors and yielded no
    /// function. ADR-0005 does not count it as lost: recovery alone does not
    /// prove a failure, and headers often parse that way.
    pub(crate) fn recover_no_functions(&mut self, file: &Path) {
        let file = file.to_string_lossy().into_owned();
        eprintln!("Warning: {file} has syntax errors, and no function was recovered from it");
        self.recovered_no_functions.push(file);
    }

    pub(crate) fn is_lost(&self, file: &str) -> bool {
        self.lost.iter().any(|l| l.file == file)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.lost.is_empty()
    }

    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    pub(crate) fn analysed(&self) -> usize {
        self.selected - self.lost.len()
    }

    pub(crate) fn lost(&self) -> &[LostInput] {
        &self.lost
    }

    pub(crate) fn recovered_no_functions(&self) -> &[String] {
        &self.recovered_no_functions
    }

    /// The `--completion-report` document.
    pub(crate) fn report(&self) -> serde_json::Value {
        let mut recovered = self.recovered_no_functions.clone();
        recovered.sort();
        let mut lost = self.lost.clone();
        lost.sort_by(|a, b| a.file.cmp(&b.file));
        json!({
            "complete": self.is_complete(),
            "files": {
                "selected": self.selected,
                "analysed": self.analysed(),
                "not_analysed": self.lost.len(),
                "recovered_no_functions": recovered.len(),
            },
            "not_analysed": lost,
            "recovered_no_functions": recovered,
        })
    }

    fn write_report(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(&self.report())?;
        fs::write(path, text + "\n")
            .with_context(|| format!("Failed to write completion report: {}", path.display()))
    }
}

/// The error an incomplete scan ends with; `main` maps it to
/// [`INCOMPLETE_EXIT`].
#[derive(Debug)]
pub(crate) struct IncompleteScan {
    lost: usize,
    selected: usize,
}

impl fmt::Display for IncompleteScan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Incomplete scan: {} of {} selected file(s) were not analysed",
            self.lost, self.selected
        )
    }
}

impl std::error::Error for IncompleteScan {}

/// Ends a scan. Writes the completion report when one was asked for, then
/// returns the gate's result, unless the scan is incomplete: then the gate's
/// error, if any, is still printed, and the scan ends as [`IncompleteScan`].
pub(crate) fn finish(
    completion: &Completion,
    report: Option<&Path>,
    gate: Result<()>,
) -> Result<()> {
    if let Some(path) = report {
        completion.write_report(path)?;
    }
    if completion.is_complete() {
        return gate;
    }
    if let Err(err) = gate {
        eprintln!("Error: {err:?}");
    }
    Err(IncompleteScan {
        lost: completion.lost.len(),
        selected: completion.selected,
    }
    .into())
}

/// Runs one file's `work`, turning a panic into that file's loss so the rest
/// of the scan survives it.
pub(crate) fn contain<T>(file: &Path, work: impl FnOnce() -> T) -> Result<T, LostInput> {
    panic::catch_unwind(AssertUnwindSafe(work))
        .map_err(|payload| LostInput::new(file, Stage::Crash, panic_message(payload.as_ref())))
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    let text = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown cause".to_string());
    format!("analysis panicked: {text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_loses_only_its_own_file() {
        let lost = contain(Path::new("a.c"), || -> u32 { panic!("boom") }).unwrap_err();
        assert_eq!(lost.stage, Stage::Crash);
        assert_eq!(lost.file, "a.c");
        assert_eq!(lost.reason, "analysis panicked: boom");
        assert_eq!(contain(Path::new("b.c"), || 7).unwrap(), 7);
    }

    #[test]
    fn a_file_lost_twice_counts_once() {
        let mut c = Completion::new(2, Vec::new());
        c.lose(LostInput::new(Path::new("a.c"), Stage::Read, "x"));
        c.lose(LostInput::new(Path::new("a.c"), Stage::Crash, "y"));
        assert_eq!((c.selected(), c.analysed(), c.lost().len()), (2, 1, 1));
    }

    #[test]
    fn an_incomplete_scan_outranks_the_gate() {
        let mut c = Completion::new(1, Vec::new());
        assert!(finish(&c, None, Ok(())).is_ok());
        c.lose(LostInput::new(Path::new("a.c"), Stage::Parse, "x"));
        let err = finish(&c, None, Err(anyhow::anyhow!("violations"))).unwrap_err();
        assert!(err.is::<IncompleteScan>());
    }

    #[test]
    fn files_lost_before_the_scan_count_as_selected() {
        let c = Completion::new(
            1,
            vec![LostInput::new(Path::new("a.txt"), Stage::Unsupported, "x")],
        );
        assert_eq!((c.selected(), c.analysed()), (2, 1));
    }
}
