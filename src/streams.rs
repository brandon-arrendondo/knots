//! Writing to stdout and stderr when their reader may have gone.
//!
//! A reader that closes the pipe early, as `knots -r src | head` or
//! `knots -r src 2>&1 | head` does, has stopped listening; the scan and its
//! verdict are unchanged. std's print macros panic on that error, so knots
//! prints through this module instead (main.rs replaces `println!`, `print!`,
//! `eprintln!` and `eprint!` with [`print_fmt`]): the first write that finds a
//! stream's pipe closed marks that stream closed, later writes to it are
//! dropped, and the run exits with the status its scan earns, 0, 1 or 3, as
//! if everything had been read.

use std::fmt;
use std::io::{self, ErrorKind, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// The stream a macro writes to.
#[derive(Clone, Copy)]
pub(crate) enum Stream {
    Stdout,
    Stderr,
}

static STDOUT_CLOSED: AtomicBool = AtomicBool::new(false);
static STDERR_CLOSED: AtomicBool = AtomicBool::new(false);

impl Stream {
    fn closed(self) -> &'static AtomicBool {
        match self {
            Stream::Stdout => &STDOUT_CLOSED,
            Stream::Stderr => &STDERR_CLOSED,
        }
    }

    fn write(self, args: fmt::Arguments<'_>, newline: bool) -> io::Result<()> {
        fn put(out: &mut impl Write, args: fmt::Arguments<'_>, newline: bool) -> io::Result<()> {
            out.write_fmt(args)?;
            if newline {
                out.write_all(b"\n")?;
            }
            Ok(())
        }
        match self {
            Stream::Stdout => put(&mut io::stdout().lock(), args, newline),
            Stream::Stderr => put(&mut io::stderr().lock(), args, newline),
        }
    }
}

/// Writes `args` (and a newline, when `newline`) to `stream`, unless its
/// reader has gone. Any other write error panics, as std's macros do.
pub(crate) fn print_fmt(stream: Stream, args: fmt::Arguments<'_>, newline: bool) {
    if stream.closed().load(Ordering::Relaxed) {
        return;
    }
    if let Err(err) = stream.write(args, newline) {
        if err.kind() == ErrorKind::BrokenPipe {
            stream.closed().store(true, Ordering::Relaxed);
        } else {
            let name = match stream {
                Stream::Stdout => "stdout",
                Stream::Stderr => "stderr",
            };
            panic!("failed printing to {name}: {err}");
        }
    }
}

/// Whether `err`, from writing a structured report, is the reader closing
/// stdout; if so, stdout is marked closed.
pub(crate) fn is_closed_pipe(err: &anyhow::Error) -> bool {
    let closed = err.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|e| e.kind() == ErrorKind::BrokenPipe)
            || cause
                .downcast_ref::<serde_json::Error>()
                .is_some_and(|e| e.io_error_kind() == Some(ErrorKind::BrokenPipe))
    });
    if closed {
        STDOUT_CLOSED.store(true, Ordering::Relaxed);
    }
    closed
}
