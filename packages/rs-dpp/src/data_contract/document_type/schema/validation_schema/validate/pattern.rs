//! A `pattern` of a validation schema as the native validator holds it: checked to parse when
//! the validator is built, and compiled the first time a value reaches it.

use super::super::{converted_regex, validator_regex_parses};
use regex::Regex;
use std::fmt;
use std::sync::OnceLock;

/// A `pattern` rewritten as the JSON Schema validator rewrites it ([`converted_regex`]), and
/// compiled with the `regex` crate under that crate's default size limit the first time a
/// value reaches it, then kept. The document validator compiles a contract it loaded from
/// state with default `regex` options, and every node validates documents against a contract
/// so loaded, so the default is the limit consensus matches under.
///
/// The validator is shared across threads, and the compile is its only interior
/// mutability: it runs once, and every thread reads what it produced. Compiling is a pure
/// function of the rewritten pattern, so when the pattern is compiled, and by which thread,
/// changes no verdict.
#[derive(Clone)]
pub(super) struct PatternCell {
    /// The pattern with the JSON Schema validator's rewrites applied.
    converted: String,
    /// The compiled pattern, `None` when the `regex` crate does not compile it: unset until a
    /// value first reaches the pattern.
    compiled: OnceLock<Option<Regex>>,
}

impl PatternCell {
    /// `pattern`, uncompiled, or `None` when it does not parse, so that the `regex` crate
    /// never compiles it ([`validator_regex_parses`]).
    pub(super) fn parsed(pattern: &str) -> Option<Self> {
        validator_regex_parses(pattern).then(|| Self {
            converted: converted_regex(pattern),
            compiled: OnceLock::new(),
        })
    }

    /// The compiled pattern, compiled by the first call and kept: `None` when the `regex`
    /// crate does not compile it.
    pub(super) fn regex(&self) -> Option<&Regex> {
        self.compiled
            .get_or_init(|| Regex::new(&self.converted).ok())
            .as_ref()
    }

    /// Whether the pattern has been compiled, or found not to compile.
    #[cfg(test)]
    pub(super) fn attempted(&self) -> bool {
        self.compiled.get().is_some()
    }
}

/// The rewritten pattern alone: whether it has been compiled yet is no part of what the
/// validator checks.
impl fmt::Debug for PatternCell {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PatternCell")
            .field(&self.converted)
            .finish()
    }
}
