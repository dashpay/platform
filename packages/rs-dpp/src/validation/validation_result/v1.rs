//! v1 of [`flatten`] / [`merge_many`].
//!
//! Canonical aggregator semantics: return `data: None` when no input
//! contributed any data (i.e. every input was either `data: None` or
//! `data: Some(empty_vec)`), and `data: Some(merged_vec)` when at least one
//! input contributed non-empty data.
//!
//! This honors the invariant `data.is_none() ⇔ no work done`, which
//! downstream code (e.g. `process_validation_result_v0:241`) relies on to
//! choose between `PaidConsensusError` and `UnpaidConsensusError`.
//!
//! See issue #2867 for context.

use super::ValidationResult;
use std::fmt::Debug;

pub(super) fn flatten<TData, E, I>(items: I) -> ValidationResult<Vec<TData>, E>
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
    if aggregate_data.is_empty() {
        ValidationResult::new_with_errors(aggregate_errors)
    } else {
        ValidationResult::new_with_data_and_errors(aggregate_data, aggregate_errors)
    }
}

pub(super) fn merge_many<TData, E, I>(items: I) -> ValidationResult<Vec<TData>, E>
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
