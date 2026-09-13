//! Merging attribute and builder declarations of one item.
//!
//! Two specs with the same identity are one item when they come from
//! different origins and agree; a builder may restate what an attribute
//! declared. Two specs of the same origin with one identity are a duplicate,
//! and two specs of different origins that disagree are a conflict.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::declare::DeclarationOrigin;
use crate::validate::diagnostic::{DeclarationPath, Diagnostic, DiagnosticKind};

/// How two specs sharing an identity relate.
pub(super) enum Overlap {
    /// Same origin: a duplicate.
    Duplicate,
    /// Different origins, same content: keep the first.
    Restatement,
    /// Different origins, different content.
    Conflict,
}

/// Classifies `kept` against `next`.
pub(super) fn classify(
    kept_origin: DeclarationOrigin,
    next_origin: DeclarationOrigin,
    equivalent: bool,
) -> Overlap {
    if kept_origin == next_origin {
        Overlap::Duplicate
    } else if equivalent {
        Overlap::Restatement
    } else {
        Overlap::Conflict
    }
}

/// Deduplicates `items` by identity, reporting duplicates and conflicts, and
/// returns the kept specs in first-seen order. `merge` is called on a
/// restatement so the caller can union what may legitimately extend (the
/// indexes of a collection).
#[allow(clippy::too_many_arguments)]
pub(super) fn dedupe<T: Clone>(
    items: &[T],
    same_identity: impl Fn(&T, &T) -> bool,
    origin: impl Fn(&T) -> DeclarationOrigin,
    equivalent: impl Fn(&T, &T) -> bool,
    merge: impl Fn(&mut T, &T),
    path: impl Fn(&T) -> DeclarationPath,
    what: &str,
    duplicate: impl Fn() -> DiagnosticKind,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<T> {
    let mut kept: Vec<T> = Vec::new();
    for item in items {
        match kept
            .iter_mut()
            .find(|existing| same_identity(existing, item))
        {
            None => kept.push(item.clone()),
            Some(existing) => {
                match classify(origin(existing), origin(item), equivalent(existing, item)) {
                    Overlap::Duplicate => {
                        diagnostics.push(Diagnostic::new(path(item), duplicate()))
                    }
                    Overlap::Restatement => merge(existing, item),
                    Overlap::Conflict => diagnostics.push(Diagnostic::new(
                        path(item),
                        DiagnosticKind::ConflictingDeclaration {
                            what: what.to_string(),
                            first: origin(existing),
                            second: origin(item),
                        },
                    )),
                }
            }
        }
    }
    kept
}
