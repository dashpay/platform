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
//! # Caller-intent ambiguity
//!
//! `merge_many_v1` keys on `aggregate_data.is_empty()` to decide between
//! `data: None` and `data: Some(_)`. Every `Some(_)` input contributes one
//! element to `aggregate_data`, so the only way to get `data: None` is to
//! have zero inputs with `data: Some(_)`. There is no `Some(empty_vec)`
//! input shape at this layer (the per-item `data` is `TData`, not
//! `Vec<TData>`), so the collapse hazard described for `flatten_v1`
//! doesn't apply here. The dispatcher facade ([`ValidationResult::merge_many`])
//! shares the limitation note for symmetry.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_non_empty_data() {
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_data(1);
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_data(2);
        let r3: ValidationResult<i32, String> = ValidationResult::new_with_error("e".to_string());

        let merged = merge_many_v1(vec![r1, r2, r3]);
        assert_eq!(merged.data, Some(vec![1, 2]));
        assert_eq!(merged.errors, vec!["e".to_string()]);
    }

    #[test]
    fn empty_input_returns_none() {
        let merged: ValidationResult<Vec<i32>, String> =
            merge_many_v1(std::iter::empty::<ValidationResult<i32, String>>());
        assert!(merged.data.is_none());
        assert!(merged.errors.is_empty());
    }

    #[test]
    fn all_inputs_no_data_returns_none() {
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_error("e2".to_string());

        let merged = merge_many_v1(vec![r1, r2]);
        assert!(merged.data.is_none());
        assert_eq!(merged.errors, vec!["e1".to_string(), "e2".to_string()]);
    }

    #[test]
    fn some_data_returns_some() {
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_data(7);

        let merged = merge_many_v1(vec![r1, r2]);
        assert_eq!(merged.data, Some(vec![7]));
        assert_eq!(merged.errors, vec!["e1".to_string()]);
    }
}
