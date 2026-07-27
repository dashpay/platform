//! `SELECT` projection types for the v1 `getDocuments` surface.
//!
//! The projection determines what the response carries:
//! - [`SelectFunction::Documents`]: matched rows (`ResultData.documents`).
//! - [`SelectFunction::Count`]: row counts — single aggregate when
//!   `group_by` is empty, per-group entries otherwise.
//! - [`SelectFunction::Sum`] / [`SelectFunction::Avg`]: numeric
//!   aggregate(s) of a named field — same single/per-group shape
//!   contract as `Count`.
//!
//! Per-function `field` requirements live in
//! [`SelectProjection::field`]; the type itself just carries the
//! `(function, field)` pair.
//!
//! Shared between the wire-decoding layer
//! (`rs-drive-abci/src/query/document_query/v1/conversions.rs`)
//! and the SDK's request builder
//! (`rs-sdk/src/platform/documents/document_query.rs`). Server
//! capability today: [`SelectFunction::Documents`],
//! [`SelectFunction::Count`] with empty `field` (= `COUNT(*)`),
//! [`SelectFunction::Sum`], and [`SelectFunction::Avg`] route
//! through the drive count / sum / average dispatchers and are
//! evaluated end-to-end (no-proof and proof paths).
//! [`SelectFunction::Count`] with non-empty `field` (=
//! `COUNT(field)`), [`SelectFunction::Min`], and
//! [`SelectFunction::Max`] are wire-stable but rejected at routing
//! time with `QuerySyntaxError::Unsupported("SELECT … is not yet
//! implemented")` — the surface is shipped first so callers can
//! encode against it, with execution landing later without
//! another version bump.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Projection function applied by `SELECT`. Distinct from
/// [`crate::query::HavingAggregateFunction`] because this enum
/// carries the document-fetch branch too — `SELECT` may return
/// rows, not just aggregates — so the two can't share a type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SelectFunction {
    /// Return matched rows. The default so old fixtures /
    /// builders that don't opt in get plain document-fetch
    /// semantics.
    #[default]
    Documents,
    /// `COUNT(*)` when [`SelectProjection::field`] is empty,
    /// `COUNT(field)` (count of non-null `field` values)
    /// otherwise. Empty-`field` form is currently the only
    /// supported COUNT shape — the `field`-bearing form is
    /// reserved for future server capability.
    Count,
    /// `SUM(field)`. Required field; numeric typed. Routed
    /// end-to-end through the drive sum dispatcher (no-proof and
    /// proof paths both terminate at grovedb's aggregate-sum /
    /// sum-tree-walk primitives — see
    /// `crate::query::drive_document_sum_query`).
    Sum,
    /// `AVG(field)`. Required field; numeric typed. Result is
    /// `f64`. Routed end-to-end through the drive average
    /// dispatcher, which composes count and sum walks under the
    /// hood (no separate average primitive on the grovedb side) —
    /// see `crate::query::drive_document_average_query`.
    Avg,
    /// `MIN(field)` — smallest value of `field` in each group
    /// (or across all matching rows when `group_by` is empty).
    /// Required field. Currently always rejected with "not yet
    /// implemented"; **semantically distinct** from
    /// [`crate::query::HavingRankingKind::Min`] which is a
    /// cross-group ranking primitive on the HAVING right side.
    Min,
    /// `MAX(field)` — symmetric to [`Self::Min`] for the largest
    /// value. Same not-yet-implemented contract; same caveat
    /// versus [`crate::query::HavingRankingKind::Max`].
    Max,
}

/// `(function, field)` projection. The `field` semantics depend
/// on `function`:
/// - [`SelectFunction::Documents`]: `field` must be empty.
/// - [`SelectFunction::Count`]: empty `field` means `COUNT(*)`;
///   non-empty means `COUNT(field)` (count of non-null values).
/// - [`SelectFunction::Sum`] / [`SelectFunction::Avg`]: `field`
///   is required and must be numeric-typed.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SelectProjection {
    /// Projection function.
    pub function: SelectFunction,
    /// Field the function is applied to. See
    /// [`SelectFunction`]'s per-variant docstring for the
    /// per-function requirement.
    pub field: String,
}

impl SelectProjection {
    /// Plain document fetch — the default projection. `field` is
    /// empty.
    pub fn documents() -> Self {
        Self {
            function: SelectFunction::Documents,
            field: String::new(),
        }
    }

    /// `COUNT(*)` — empty `field`.
    pub fn count_star() -> Self {
        Self {
            function: SelectFunction::Count,
            field: String::new(),
        }
    }

    /// `COUNT(field)` — count of non-null values of `field`.
    pub fn count_field(field: impl Into<String>) -> Self {
        Self {
            function: SelectFunction::Count,
            field: field.into(),
        }
    }

    /// `SUM(field)`.
    pub fn sum(field: impl Into<String>) -> Self {
        Self {
            function: SelectFunction::Sum,
            field: field.into(),
        }
    }

    /// `AVG(field)`.
    pub fn avg(field: impl Into<String>) -> Self {
        Self {
            function: SelectFunction::Avg,
            field: field.into(),
        }
    }

    /// `MIN(field)` — per-group / global minimum.
    pub fn min(field: impl Into<String>) -> Self {
        Self {
            function: SelectFunction::Min,
            field: field.into(),
        }
    }

    /// `MAX(field)` — per-group / global maximum.
    pub fn max(field: impl Into<String>) -> Self {
        Self {
            function: SelectFunction::Max,
            field: field.into(),
        }
    }
}
