//! v1 of [`ValidationResult::merge_many`].
//!
//! Canonical semantics: returns `data: None` when no input had
//! `data: Some(_)`, and `data: Some(Vec<TData>)` when at least one input
//! contributed data.
//!
//! This honors the invariant `data.is_none() ⇔ no work done`, which
//! downstream code (e.g. `process_validation_result_v0:241`) relies on to
//! choose between `PaidConsensusError` and `UnpaidConsensusError`.
//!
//! See issue #2867 for context.
//!
//! [`ValidationResult::merge_many`]: crate::validation::ValidationResult::merge_many

use crate::validation::ValidationResult;
use std::fmt::Debug;

pub(in crate::validation::validation_result) fn merge_many_v1<TData, E, I>(
    items: I,
) -> ValidationResult<Vec<TData>, E>
where
    TData: Clone,
    E: Debug,
    I: IntoIterator<Item = ValidationResult<TData, E>>,
{
    let mut aggregate_errors = vec![];
    let mut aggregate_data = vec![];
    items.into_iter().for_each(|single_validation_result| {
        let ValidationResult { mut errors, data } = single_validation_result;
        aggregate_errors.append(&mut errors);
        if let Some(data) = data {
            aggregate_data.push(data);
        }
    });
    if aggregate_data.is_empty() {
        ValidationResult::new_with_errors(aggregate_errors)
    } else {
        ValidationResult::new_with_data_and_errors(aggregate_data, aggregate_errors)
    }
}
