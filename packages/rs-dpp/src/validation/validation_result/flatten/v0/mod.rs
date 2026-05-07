//! v0 of [`ConsensusValidationResult::flatten`].
//!
//! Legacy semantics: always returns `data: Some(Vec<...>)`, including
//! `Some(empty_vec)` when no input contributed any data.
//!
//! Preserved for `PROTOCOL_VERSION_11` and below — the
//! `Some(empty_vec)`-on-no-data behavior is part of the existing chain
//! history, and changing it would be a consensus-breaking change for
//! already-finalized blocks. New code should let the facade dispatch to v1.
//!
//! See issue #2867 for context.
//!
//! [`ConsensusValidationResult::flatten`]: crate::validation::ConsensusValidationResult::flatten

use crate::validation::ValidationResult;
use std::fmt::Debug;

pub(in crate::validation::validation_result) fn flatten_v0<TData, E, I>(
    items: I,
) -> ValidationResult<Vec<TData>, E>
where
    TData: Clone,
    E: Debug,
    I: IntoIterator<Item = ValidationResult<Vec<TData>, E>>,
{
    let mut aggregate_errors = vec![];
    let mut aggregate_data = vec![];
    items.into_iter().for_each(|single_validation_result| {
        let ValidationResult { mut errors, data } = single_validation_result;
        aggregate_errors.append(&mut errors);
        if let Some(mut data) = data {
            aggregate_data.append(&mut data);
        }
    });
    ValidationResult::new_with_data_and_errors(aggregate_data, aggregate_errors)
}
