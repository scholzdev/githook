//! Runtime error type with source-location tracking.
//!
//! [`EvalError`] wraps an error message together with an optional [`Span`]
//! so that the CLI and LSP can display precise source locations for runtime
//! failures (e.g. "variable not found at line 12, column 5").

use githook_syntax::error::Span;
use std::fmt;

/// A runtime evaluation error that carries an optional source [`Span`].
///
/// Use the [`bail_span!`] macro (or [`EvalError::new`]) to construct these
/// inside the executor.  The outer [`anyhow::Error`] wrapper is preserved so
/// that call-sites can keep using `Result<T>` without changing every
/// function signature.
#[derive(Debug, Clone)]
pub struct EvalError {
    /// Human-readable error description.
    pub message: String,
    /// Source location where the error originated (if available).
    pub span: Option<Span>,
}

impl EvalError {
    /// Creates a new evaluation error with a span.
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// Creates a new evaluation error with a span reference.
    pub fn spanned(message: impl Into<String>, span: &Span) -> Self {
        Self {
            message: message.into(),
            span: Some(*span),
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EvalError {}

/// Bail out of a function with an [`EvalError`] that includes a source span.
///
/// # Usage
/// ```ignore
/// bail_span!(span, "Variable '{}' not found", name);
/// bail_span!(None::<githook_syntax::error::Span>, "Division by zero");
/// ```
#[macro_export]
macro_rules! bail_span {
    ($span:expr, $($arg:tt)*) => {
        return Err(anyhow::anyhow!($crate::error::EvalError::new(
            format!($($arg)*),
            $crate::error::into_option_span($span),
        )))
    };
}

/// Helper to convert various span representations into `Option<Span>`.
///
/// Accepts `Span`, `&Span`, `Option<Span>`, and `Option<&Span>`.
pub fn into_option_span(span: impl IntoOptionSpan) -> Option<Span> {
    span.into_option_span()
}

/// Trait for converting span-like values to `Option<Span>`.
pub trait IntoOptionSpan {
    /// Converts into `Option<Span>`.
    fn into_option_span(self) -> Option<Span>;
}

impl IntoOptionSpan for Span {
    fn into_option_span(self) -> Option<Span> {
        Some(self)
    }
}

impl IntoOptionSpan for &Span {
    fn into_option_span(self) -> Option<Span> {
        Some(*self)
    }
}

impl IntoOptionSpan for Option<Span> {
    fn into_option_span(self) -> Option<Span> {
        self
    }
}

impl IntoOptionSpan for Option<&Span> {
    fn into_option_span(self) -> Option<Span> {
        self.copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_error_new_with_span() {
        let span = Span::new(1, 5, 4, 10);
        let err = EvalError::new("test error", Some(span));
        assert_eq!(err.message, "test error");
        assert_eq!(err.span, Some(span));
    }

    #[test]
    fn eval_error_new_without_span() {
        let err = EvalError::new("no span", None);
        assert_eq!(err.message, "no span");
        assert!(err.span.is_none());
    }

    #[test]
    fn eval_error_spanned() {
        let span = Span::new(3, 1, 20, 25);
        let err = EvalError::spanned("spanned error", &span);
        assert_eq!(err.message, "spanned error");
        assert_eq!(err.span, Some(span));
    }

    #[test]
    fn eval_error_display() {
        let err = EvalError::new("display test", None);
        assert_eq!(format!("{err}"), "display test");
    }

    #[test]
    fn eval_error_display_ignores_span() {
        let span = Span::new(1, 1, 0, 5);
        let err = EvalError::new("msg", Some(span));
        // Display only shows message, not span
        assert_eq!(format!("{err}"), "msg");
    }

    #[test]
    fn eval_error_is_std_error() {
        let err = EvalError::new("std error", None);
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn eval_error_downcast_from_anyhow() {
        let span = Span::new(2, 3, 10, 15);
        let anyhow_err = anyhow::anyhow!(EvalError::new("wrapped", Some(span)));
        let downcast = anyhow_err.downcast_ref::<EvalError>().unwrap();
        assert_eq!(downcast.message, "wrapped");
        assert_eq!(downcast.span, Some(span));
    }

    #[test]
    fn into_option_span_from_span() {
        let span = Span::new(1, 1, 0, 1);
        let opt = into_option_span(span);
        assert_eq!(opt, Some(span));
    }

    #[test]
    fn into_option_span_from_ref() {
        let span = Span::new(1, 1, 0, 1);
        let span_ref: &Span = &span;
        let opt = into_option_span(span_ref);
        assert_eq!(opt, Some(span));
    }

    #[test]
    fn into_option_span_from_option_some() {
        let span = Span::new(1, 1, 0, 1);
        let opt = into_option_span(Some(span));
        assert_eq!(opt, Some(span));
    }

    #[test]
    fn into_option_span_from_option_none() {
        let opt = into_option_span(None::<Span>);
        assert!(opt.is_none());
    }

    #[test]
    fn into_option_span_from_option_ref() {
        let span = Span::new(1, 1, 0, 1);
        let opt = into_option_span(Some(&span));
        assert_eq!(opt, Some(span));
    }

    #[test]
    fn bail_span_macro_produces_eval_error() {
        fn try_bail() -> anyhow::Result<()> {
            let span = Span::new(5, 10, 40, 50);
            bail_span!(&span, "variable '{}' not found", "x");
        }
        let err = try_bail().unwrap_err();
        let eval_err = err.downcast_ref::<EvalError>().unwrap();
        assert_eq!(eval_err.message, "variable 'x' not found");
        assert_eq!(eval_err.span, Some(Span::new(5, 10, 40, 50)));
    }

    #[test]
    fn bail_span_macro_with_none_span() {
        fn try_bail() -> anyhow::Result<()> {
            bail_span!(None::<Span>, "division by zero");
        }
        let err = try_bail().unwrap_err();
        let eval_err = err.downcast_ref::<EvalError>().unwrap();
        assert_eq!(eval_err.message, "division by zero");
        assert!(eval_err.span.is_none());
    }
}
