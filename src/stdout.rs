//! Writing the report to stdout when its reader may have gone.
//!
//! A reader that closes the pipe early, as `knots -r src | head` does, has
//! stopped listening; the scan and its verdict are unchanged. std's print
//! macros panic on that error, so knots prints through this module instead
//! (main.rs replaces `println!` and `print!` with [`print_fmt`]): the first
//! write that finds the pipe closed marks stdout closed, later writes are
//! dropped, and the run exits with the status its scan earns, 0, 1 or 3, as
//! if the report had been read. Threshold violations still go to stderr.

use std::fmt;
use std::io::{self, ErrorKind, Write};
use std::sync::atomic::{AtomicBool, Ordering};

static CLOSED: AtomicBool = AtomicBool::new(false);

/// Writes `args` (and a newline, when `newline`) to stdout, unless the reader
/// has gone. Any other write error panics, as `println!` does.
pub(crate) fn print_fmt(args: fmt::Arguments<'_>, newline: bool) {
    if CLOSED.load(Ordering::Relaxed) {
        return;
    }
    let mut out = io::stdout().lock();
    let written = out.write_fmt(args).and_then(|()| {
        if newline {
            out.write_all(b"\n")
        } else {
            Ok(())
        }
    });
    if let Err(err) = written {
        if !mark_if_closed(&err) {
            panic!("failed printing to stdout: {err}");
        }
    }
}

/// Whether `err`, from writing a structured report, is the reader closing
/// stdout; if so, stdout is marked closed.
pub(crate) fn is_closed_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        let io_error = cause
            .downcast_ref::<io::Error>()
            .is_some_and(mark_if_closed);
        let json_error = cause
            .downcast_ref::<serde_json::Error>()
            .is_some_and(|e| e.io_error_kind() == Some(ErrorKind::BrokenPipe) && mark_closed());
        io_error || json_error
    })
}

fn mark_if_closed(err: &io::Error) -> bool {
    err.kind() == ErrorKind::BrokenPipe && mark_closed()
}

fn mark_closed() -> bool {
    CLOSED.store(true, Ordering::Relaxed);
    true
}
