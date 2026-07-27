//! v1 of [`ConsensusValidationResult::flatten`].
//!
//! Canonical semantics: returns `data: None` when no input contributed any
//! data (i.e. every input was either `data: None` or `data: Some(empty_vec)`),
//! and `data: Some(merged_vec)` when at least one input contributed
//! non-empty data.
//!
//! This honors the invariant `data.is_none() ⇔ no work done`, which
//! downstream code (e.g. `process_validation_result_v0:241`) relies on to
//! choose between `PaidConsensusError` and `UnpaidConsensusError`.
//!
//! # Caller-intent ambiguity
//!
//! `flatten_v1` keys on `aggregate_data.is_empty()` to decide between
//! `data: None` and `data: Some(_)`. This collapses two distinct
//! caller-side intents into the same output:
//!
//! * **Truly no work**: every input had `data: None`.
//! * **Validated but produced no output**: every input had
//!   `data: Some(empty_vec)`.
//!
//! v1 cannot distinguish those two cases at the aggregate level — both
//! end up as `data: None` and are routed to `UnpaidConsensusError`
//! downstream. For the documents-batch path under PROTOCOL_VERSION_12 this
//! is safe: every per-transition handler emits at least one action on
//! success and a bump action on failure, so no caller produces
//! `Some(empty_vec)`. A future caller that needs "validated, but no
//! actions to apply" must signal that with at least one non-empty entry,
//! not with `Some(empty_vec)`.
//!
//! See issue #2867 for context.
//!
//! [`ConsensusValidationResult::flatten`]: crate::validation::ConsensusValidationResult::flatten

use crate::validation::ValidationResult;
use std::fmt::Debug;

pub(in crate::validation::validation_result) fn flatten_v1<TData, E, I>(
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
    fn merges_non_empty_data() {
        let r1: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![1, 2]);
        let r2: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_data_and_errors(vec![3], vec!["e".to_string()]);
        let r3: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e2".to_string());

        let flat = flatten_v1(vec![r1, r2, r3]);
        assert_eq!(flat.data, Some(vec![1, 2, 3]));
        assert_eq!(flat.errors, vec!["e".to_string(), "e2".to_string()]);
    }

    #[test]
    fn empty_input_returns_none() {
        let flat: ValidationResult<Vec<i32>, String> =
            flatten_v1(std::iter::empty::<ValidationResult<Vec<i32>, String>>());
        assert_eq!(flat.data, None);
        assert!(flat.errors.is_empty());
    }

    #[test]
    fn all_inputs_no_data_returns_none() {
        // Downstream code (process_validation_result_v0:241) keys on
        // data.is_none() to route to UnpaidConsensusError.
        let r1: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e2".to_string());

        let flat = flatten_v1(vec![r1, r2]);
        assert!(flat.data.is_none());
        assert_eq!(flat.errors, vec!["e1".to_string(), "e2".to_string()]);
    }

    #[test]
    fn some_empty_some_non_empty_returns_some() {
        let r1: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![]);
        let r2: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![42]);

        let flat = flatten_v1(vec![r1, r2]);
        assert_eq!(flat.data, Some(vec![42]));
        assert!(flat.errors.is_empty());
    }

    #[test]
    fn all_some_empty_returns_none() {
        // All inputs had data:Some(empty_vec). The aggregate Vec is empty → data:None.
        let r1: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![]);
        let r2: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![]);

        let flat = flatten_v1(vec![r1, r2]);
        assert!(flat.data.is_none());
        assert!(flat.errors.is_empty());
    }
}
