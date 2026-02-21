//! Structured compiler diagnostics — §23.2 of the architecture spec.
//!
//! Every error Rava emits must include filename, line number, and a fix suggestion.
//! This mirrors the Rust compiler's diagnostic format.
//!
//! Example output:
//! ```text
//! error[E0042]: cannot resolve reflection target at compile time
//!   --> src/com/example/Service.java:42:18
//!    |
//! 42 |     Class.forName(config.get("class"))
//!    |                   ^^^^^^^^^^^^^^^^^^^ dynamic string, cannot resolve
//!    |
//!    = note: this call will be handled by Rava MicroRT at runtime
//!    = help: if performance is critical, use a compile-time constant string
//! ```

use crate::span::Span;

/// Severity level of a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
}

/// A structured compiler diagnostic.
///
/// Every diagnostic must include a span (location) and a help suggestion.
/// No diagnostic may be emitted without a fix suggestion — this is enforced
/// by the constructor requiring `help`.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level:   DiagnosticLevel,
    /// Short error code, e.g. `"E0042"`.
    pub code:    &'static str,
    /// Human-readable description of the problem.
    pub message: String,
    /// Source location where the problem was detected.
    pub span:    Span,
    /// Annotation displayed inline at the span (what is wrong here).
    pub label:   Option<String>,
    /// Fix suggestion — how to resolve the problem.
    pub help:    Option<String>,
    /// Additional context notes.
    pub notes:   Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            level:   DiagnosticLevel::Error,
            code,
            message: message.into(),
            span,
            label:   None,
            help:    None,
            notes:   Vec::new(),
        }
    }

    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            level:   DiagnosticLevel::Warning,
            code,
            message: message.into(),
            span,
            label:   None,
            help:    None,
            notes:   Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// Extension point: how diagnostics are rendered to the user.
///
/// Default: `TerminalEmitter` (colored terminal output).
/// Alternative: JSON emitter for IDE integration.
pub trait DiagnosticEmitter: Send + Sync {
    fn emit(&self, diagnostic: &Diagnostic);
    fn emit_all(&self, diagnostics: &[Diagnostic]) {
        for d in diagnostics {
            self.emit(d);
        }
    }
}

/// Simple terminal emitter — prints diagnostics to stderr.
pub struct TerminalEmitter;

impl DiagnosticEmitter for TerminalEmitter {
    fn emit(&self, d: &Diagnostic) {
        let level = match d.level {
            DiagnosticLevel::Error   => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Note    => "note",
        };
        eprintln!("{}[{}]: {}", level, d.code, d.message);
        eprintln!("  --> {}", d.span);
        if let Some(ref label) = d.label {
            eprintln!("   | {}", label);
        }
        for note in &d.notes {
            eprintln!("   = note: {}", note);
        }
        if let Some(ref help) = d.help {
            eprintln!("   = help: {}", help);
        }
    }
}
