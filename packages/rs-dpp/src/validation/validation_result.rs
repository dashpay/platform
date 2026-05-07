use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use std::fmt::Debug;

#[macro_export]
macro_rules! check_validation_result_with_data {
    ($result:expr) => {
        match $result {
            Ok(result) => result,
            Err(e) => return Ok(ValidationResult::new_with_errors(vec![e.into()])),
        }
    };
}

pub type SimpleValidationResult<E> = ValidationResult<(), E>;

pub type ConsensusValidationResult<TData> = ValidationResult<TData, ConsensusError>;

pub type SimpleConsensusValidationResult = ConsensusValidationResult<()>;

#[derive(Debug, Clone)]
pub struct ValidationResult<TData: Clone, E: Debug> {
    pub errors: Vec<E>,
    pub data: Option<TData>,
}

impl<T: Clone, E: Debug> Default for ValidationResult<T, E> {
    fn default() -> Self {
        ValidationResult {
            errors: Vec::new(),
            data: None,
        }
    }
}

impl<TData: Clone, E: Debug> ValidationResult<Vec<TData>, E> {
    /// **Deprecated.** Always returns `data: Some(Vec<...>)` — even if no
    /// input contributed any data — which violates the implicit contract
    /// `data.is_none() ⇔ no work done` that downstream `process_validation_result`
    /// keys on. See issue #2867 (the empty-action / "validating state
    /// transition for free" bug). Use [`flatten_strict`] instead, which
    /// returns `data: None` when no input contributed data.
    ///
    /// Preserved for `PROTOCOL_VERSION_11` and below — changing this
    /// function's behavior would be a consensus-breaking change for the
    /// existing chain history.
    ///
    /// [`flatten_strict`]: ValidationResult::flatten_strict
    #[deprecated(
        since = "3.1.0",
        note = "use flatten_strict; flatten always returns Some(empty_vec) which violates the data-is-None ⇔ no-work invariant — see issue #2867"
    )]
    pub fn flatten<I: IntoIterator<Item = ValidationResult<Vec<TData>, E>>>(
        items: I,
    ) -> ValidationResult<Vec<TData>, E> {
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

    /// Strict variant of [`flatten`]: returns `data: None` when no input
    /// contributed any data (i.e. every input was either `data: None` or
    /// `data: Some(empty_vec)`), and only returns `data: Some(...)` when
    /// the aggregate Vec is non-empty.
    ///
    /// This restores the invariant that `data.is_none() ⇔ no work done`,
    /// which downstream code (e.g.
    /// `process_validation_result_v0:241`) relies on to choose between
    /// `PaidConsensusError` and `UnpaidConsensusError`. Used by
    /// `PROTOCOL_VERSION_12`+ to close the issue #2867 "validating state
    /// transition for free" gap.
    ///
    /// [`flatten`]: ValidationResult::flatten
    pub fn flatten_strict<I: IntoIterator<Item = ValidationResult<Vec<TData>, E>>>(
        items: I,
    ) -> ValidationResult<Vec<TData>, E> {
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
            ValidationResult {
                errors: aggregate_errors,
                data: None,
            }
        } else {
            ValidationResult::new_with_data_and_errors(aggregate_data, aggregate_errors)
        }
    }
}

impl<TData: Clone, E: Debug> ValidationResult<TData, E> {
    /// **Deprecated.** Always returns `data: Some(Vec<...>)` — even if no
    /// input contributed any data — which violates the implicit contract
    /// `data.is_none() ⇔ no work done`. See issue #2867. Use
    /// [`merge_many_strict`] instead.
    ///
    /// Preserved for `PROTOCOL_VERSION_11` and below — changing this
    /// function's behavior would be a consensus-breaking change for the
    /// existing chain history.
    ///
    /// [`merge_many_strict`]: ValidationResult::merge_many_strict
    #[deprecated(
        since = "3.1.0",
        note = "use merge_many_strict; merge_many always returns Some(empty_vec) which violates the data-is-None ⇔ no-work invariant — see issue #2867"
    )]
    pub fn merge_many<I: IntoIterator<Item = ValidationResult<TData, E>>>(
        items: I,
    ) -> ValidationResult<Vec<TData>, E> {
        let mut aggregate_errors = vec![];
        let mut aggregate_data = vec![];
        items.into_iter().for_each(|single_validation_result| {
            let ValidationResult { mut errors, data } = single_validation_result;
            aggregate_errors.append(&mut errors);
            if let Some(data) = data {
                aggregate_data.push(data);
            }
        });
        ValidationResult::new_with_data_and_errors(aggregate_data, aggregate_errors)
    }

    /// Strict variant of [`merge_many`]: returns `data: None` when no
    /// input had `Some(data)`, and only returns `data: Some(Vec<...>)`
    /// when at least one input contributed data.
    ///
    /// This restores the `data.is_none() ⇔ no work done` invariant — see
    /// issue #2867. Used by `PROTOCOL_VERSION_12`+ to close the
    /// "validating state transition for free" gap.
    ///
    /// [`merge_many`]: ValidationResult::merge_many
    pub fn merge_many_strict<I: IntoIterator<Item = ValidationResult<TData, E>>>(
        items: I,
    ) -> ValidationResult<Vec<TData>, E> {
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
            ValidationResult {
                errors: aggregate_errors,
                data: None,
            }
        } else {
            ValidationResult::new_with_data_and_errors(aggregate_data, aggregate_errors)
        }
    }
}

impl<E: Debug> SimpleValidationResult<E> {
    pub fn merge_many_errors<I: IntoIterator<Item = SimpleValidationResult<E>>>(
        items: I,
    ) -> SimpleValidationResult<E> {
        let errors = items
            .into_iter()
            .flat_map(|single_validation_result| single_validation_result.errors)
            .collect();
        SimpleValidationResult::new_with_errors(errors)
    }
}

impl<TData: Clone, E: Debug> ValidationResult<TData, E> {
    pub fn new() -> Self {
        Self {
            errors: vec![],
            data: None::<TData>,
        }
    }

    pub fn new_with_data(data: TData) -> Self {
        Self {
            errors: vec![],
            data: Some(data),
        }
    }

    pub fn new_with_data_and_errors(data: TData, errors: Vec<E>) -> Self {
        Self {
            errors,
            data: Some(data),
        }
    }

    pub fn new_with_error(error: E) -> Self {
        Self {
            errors: vec![error],
            data: None,
        }
    }

    pub fn new_with_errors(errors: Vec<E>) -> Self {
        Self { errors, data: None }
    }

    pub fn map<F, U: Clone>(self, f: F) -> ValidationResult<U, E>
    where
        F: FnOnce(TData) -> U,
    {
        ValidationResult {
            errors: self.errors,
            data: self.data.map(f),
        }
    }

    pub fn map_result<F, U: Clone, G>(self, f: F) -> Result<ValidationResult<U, E>, G>
    where
        F: FnOnce(TData) -> Result<U, G>,
    {
        Ok(ValidationResult {
            errors: self.errors,
            data: self.data.map(f).transpose()?,
        })
    }

    pub fn and_then_simple_validation<F>(
        self,
        f: F,
    ) -> Result<ValidationResult<TData, E>, ProtocolError>
    where
        F: FnOnce(&TData) -> Result<SimpleValidationResult<E>, ProtocolError>,
    {
        let new_errors = self.data.as_ref().map(f).transpose()?;
        let mut result = ValidationResult {
            errors: self.errors,
            data: self.data,
        };
        if let Some(new_errors) = new_errors {
            result.add_errors(new_errors.errors)
        }
        Ok(result)
    }

    pub fn and_then_validation<F, U: Clone, G>(self, f: F) -> Result<ValidationResult<U, E>, G>
    where
        F: FnOnce(TData) -> Result<ValidationResult<U, E>, G>,
    {
        if let Some(data) = self.data {
            let mut new_validation_result = f(data)?;
            new_validation_result.add_errors(self.errors);
            Ok(new_validation_result)
        } else {
            Ok(ValidationResult::<U, E>::new_with_errors(self.errors))
        }
    }

    pub fn and_then_borrowed_validation<F, U: Clone, G>(
        self,
        f: F,
    ) -> Result<ValidationResult<U, E>, G>
    where
        F: FnOnce(&TData) -> Result<ValidationResult<U, E>, G>,
    {
        if let Some(data) = self.data.as_ref() {
            let mut new_validation_result = f(data)?;
            new_validation_result.add_errors(self.errors);
            Ok(new_validation_result)
        } else {
            Ok(ValidationResult::<U, E>::new_with_errors(self.errors))
        }
    }

    pub fn add_error<T>(&mut self, error: T)
    where
        T: Into<E>,
    {
        self.errors.push(error.into())
    }

    pub fn add_errors(&mut self, mut errors: Vec<E>) {
        self.errors.append(&mut errors)
    }

    pub fn add_errors_into<EI: Into<E>>(&mut self, errors: Vec<EI>) {
        errors.into_iter().for_each(|e| self.add_error(e.into()))
    }

    pub fn merge<TOtherData: Clone>(&mut self, mut other: ValidationResult<TOtherData, E>) {
        self.errors.append(&mut other.errors);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn is_err(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn first_error(&self) -> Option<&E> {
        self.errors.first()
    }

    pub fn get_error(&self, pos: usize) -> Option<&E> {
        self.errors.get(pos)
    }

    pub fn into_result_without_data(self) -> ValidationResult<(), E> {
        ValidationResult {
            errors: self.errors,
            data: None,
        }
    }

    pub fn is_valid_with_data(&self) -> bool {
        self.is_valid() && self.data.is_some()
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn set_data(&mut self, data: TData) {
        self.data = Some(data)
    }

    pub fn into_data(self) -> Result<TData, ProtocolError> {
        self.data
            .ok_or(ProtocolError::CorruptedCodeExecution(format!(
                "trying to push validation result into data (errors are {:?})",
                self.errors
            )))
    }

    pub fn into_data_with_error(mut self) -> Result<Result<TData, E>, ProtocolError> {
        if let Some(error) = self.errors.pop() {
            Ok(Err(error))
        } else {
            self.data
                .map(Ok)
                .ok_or(ProtocolError::CorruptedCodeExecution(format!(
                    "trying to push validation result into data (errors are {:?})",
                    self.errors
                )))
        }
    }

    pub fn into_data_and_errors(self) -> Result<(TData, Vec<E>), ProtocolError> {
        Ok((
            self.data
                .ok_or(ProtocolError::CorruptedCodeExecution(format!(
                    "trying to push validation result into data (errors are {:?})",
                    self.errors
                )))?,
            self.errors,
        ))
    }

    pub fn data_as_borrowed(&self) -> Result<&TData, ProtocolError> {
        self.data
            .as_ref()
            .ok_or(ProtocolError::CorruptedCodeExecution(format!(
                "trying to get validation result as data (errors are {:?})",
                self.errors
            )))
    }
}

impl<TData: Clone, E: Debug> From<TData> for ValidationResult<TData, E> {
    fn from(value: TData) -> Self {
        ValidationResult::new_with_data(value)
    }
}

impl<TData: Clone, E: Debug, F: Into<E>> From<Result<TData, F>> for ValidationResult<TData, E> {
    fn from(value: Result<TData, F>) -> Self {
        match value {
            Ok(data) => ValidationResult::new_with_data(data),
            Err(e) => ValidationResult::new_with_errors(vec![e.into()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- new() --

    #[test]
    fn test_new_has_no_errors() {
        let result: ValidationResult<i32, String> = ValidationResult::new();
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_new_has_no_data() {
        let result: ValidationResult<i32, String> = ValidationResult::new();
        assert!(result.data.is_none());
    }

    // -- new_with_data() --

    #[test]
    fn test_new_with_data_stores_data() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(42);
        assert_eq!(result.data, Some(42));
        assert!(result.errors.is_empty());
    }

    // -- new_with_error() --

    #[test]
    fn test_new_with_error_stores_single_error() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_error("bad".to_string());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "bad");
        assert!(result.data.is_none());
    }

    // -- new_with_errors() --

    #[test]
    fn test_new_with_errors_stores_multiple_errors() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_errors(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.errors[0], "a");
        assert_eq!(result.errors[1], "b");
        assert!(result.data.is_none());
    }

    #[test]
    fn test_new_with_errors_empty_vec() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_errors(vec![]);
        assert!(result.errors.is_empty());
        assert!(result.data.is_none());
    }

    // -- map() --

    #[test]
    fn test_map_transforms_data() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(10);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.data, Some(20));
        assert!(mapped.errors.is_empty());
    }

    #[test]
    fn test_map_preserves_errors() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_data_and_errors(5, vec!["err".to_string()]);
        let mapped = result.map(|x| x + 1);
        assert_eq!(mapped.data, Some(6));
        assert_eq!(mapped.errors, vec!["err".to_string()]);
    }

    #[test]
    fn test_map_with_no_data() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_error("err".to_string());
        let mapped = result.map(|x| x + 1);
        assert!(mapped.data.is_none());
        assert_eq!(mapped.errors.len(), 1);
    }

    // -- map_result() --

    #[test]
    fn test_map_result_with_ok_closure() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(10);
        let mapped: Result<ValidationResult<String, String>, String> =
            result.map_result(|x| Ok(format!("val={}", x)));
        let mapped = mapped.unwrap();
        assert_eq!(mapped.data, Some("val=10".to_string()));
    }

    #[test]
    fn test_map_result_with_err_closure() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(10);
        let mapped: Result<ValidationResult<i32, String>, String> =
            result.map_result(|_| Err("fail".to_string()));
        assert!(mapped.is_err());
        assert_eq!(mapped.unwrap_err(), "fail");
    }

    #[test]
    fn test_map_result_with_no_data() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_error("err".to_string());
        let mapped: Result<ValidationResult<i32, String>, String> =
            result.map_result(|x| Ok(x + 1));
        let mapped = mapped.unwrap();
        assert!(mapped.data.is_none());
        assert_eq!(mapped.errors, vec!["err".to_string()]);
    }

    // -- is_valid() / is_err() --

    #[test]
    fn test_is_valid_true_when_no_errors() {
        let result: ValidationResult<i32, String> = ValidationResult::new();
        assert!(result.is_valid());
        assert!(!result.is_err());
    }

    #[test]
    fn test_is_valid_false_when_errors_present() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_error("e".to_string());
        assert!(!result.is_valid());
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_with_data_and_no_errors() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(1);
        assert!(result.is_valid());
    }

    #[test]
    fn test_is_err_with_data_and_errors() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_data_and_errors(1, vec!["e".to_string()]);
        assert!(result.is_err());
    }

    // -- first_error() --

    #[test]
    fn test_first_error_returns_first() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_errors(vec!["first".to_string(), "second".to_string()]);
        assert_eq!(result.first_error(), Some(&"first".to_string()));
    }

    #[test]
    fn test_first_error_returns_none_when_no_errors() {
        let result: ValidationResult<i32, String> = ValidationResult::new();
        assert_eq!(result.first_error(), None);
    }

    // -- into_data() --

    #[test]
    fn test_into_data_returns_data_when_present() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(42);
        assert_eq!(result.into_data().unwrap(), 42);
    }

    #[test]
    fn test_into_data_returns_error_when_no_data() {
        let result: ValidationResult<i32, String> = ValidationResult::new();
        assert!(result.into_data().is_err());
    }

    // -- into_data_with_error() --

    #[test]
    fn test_into_data_with_error_returns_data_when_valid() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(42);
        let inner = result.into_data_with_error().unwrap();
        assert_eq!(inner.unwrap(), 42);
    }

    #[test]
    fn test_into_data_with_error_returns_last_error_when_errors_present() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_errors(vec!["first".to_string(), "last".to_string()]);
        let inner = result.into_data_with_error().unwrap();
        assert_eq!(inner.unwrap_err(), "last");
    }

    #[test]
    fn test_into_data_with_error_returns_protocol_error_when_no_data_and_no_errors() {
        let result: ValidationResult<i32, String> = ValidationResult::new();
        assert!(result.into_data_with_error().is_err());
    }

    // -- into_data_and_errors() --

    #[test]
    fn test_into_data_and_errors_returns_both() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_data_and_errors(10, vec!["e".to_string()]);
        let (data, errors) = result.into_data_and_errors().unwrap();
        assert_eq!(data, 10);
        assert_eq!(errors, vec!["e".to_string()]);
    }

    #[test]
    fn test_into_data_and_errors_returns_empty_errors_when_valid() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(10);
        let (data, errors) = result.into_data_and_errors().unwrap();
        assert_eq!(data, 10);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_into_data_and_errors_fails_without_data() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_error("e".to_string());
        assert!(result.into_data_and_errors().is_err());
    }

    // -- From impls --

    #[test]
    fn test_from_data_creates_valid_result() {
        let result: ValidationResult<i32, String> = 42.into();
        assert_eq!(result.data, Some(42));
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_from_ok_result_creates_valid_result() {
        let ok_result: Result<i32, String> = Ok(42);
        let result: ValidationResult<i32, String> = ok_result.into();
        assert_eq!(result.data, Some(42));
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_from_err_result_creates_error_result() {
        let err_result: Result<i32, String> = Err("bad".to_string());
        let result: ValidationResult<i32, String> = err_result.into();
        assert!(result.data.is_none());
        assert_eq!(result.errors, vec!["bad".to_string()]);
    }

    // -- flatten() (deprecated) --
    // These pin the historical buggy behavior preserved for
    // PROTOCOL_VERSION_11 and below — issue #2867.

    #[test]
    #[allow(deprecated)]
    fn test_flatten_merges_data_and_errors() {
        let r1: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![1, 2]);
        let r2: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_data_and_errors(vec![3], vec!["e".to_string()]);
        let r3: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e2".to_string());

        let flat = ValidationResult::flatten(vec![r1, r2, r3]);
        assert_eq!(flat.data, Some(vec![1, 2, 3]));
        assert_eq!(flat.errors, vec!["e".to_string(), "e2".to_string()]);
    }

    #[test]
    #[allow(deprecated)]
    fn test_flatten_empty_input() {
        let flat: ValidationResult<Vec<i32>, String> =
            ValidationResult::flatten(std::iter::empty());
        // Issue #2867 root cause: flatten produces Some(empty_vec) here,
        // not None. Downstream code that checks `data.is_none()` is fooled
        // into treating "no data" as "has data".
        assert_eq!(flat.data, Some(vec![]));
        assert!(flat.errors.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn test_flatten_all_inputs_no_data_returns_some_empty() {
        // Pins the buggy v11 behavior: all inputs have data:None, but
        // flatten still produces data:Some(vec![]).
        let r1: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e2".to_string());

        let flat = ValidationResult::flatten(vec![r1, r2]);
        assert_eq!(flat.data, Some(vec![]));
        assert_eq!(flat.errors, vec!["e1".to_string(), "e2".to_string()]);
    }

    // -- merge_many() (deprecated) --

    #[test]
    #[allow(deprecated)]
    fn test_merge_many_collects_data_into_vec() {
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_data(1);
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_data(2);
        let r3: ValidationResult<i32, String> = ValidationResult::new_with_error("e".to_string());

        let merged = ValidationResult::merge_many(vec![r1, r2, r3]);
        assert_eq!(merged.data, Some(vec![1, 2]));
        assert_eq!(merged.errors, vec!["e".to_string()]);
    }

    #[test]
    #[allow(deprecated)]
    fn test_merge_many_empty_input() {
        let merged: ValidationResult<Vec<i32>, String> =
            ValidationResult::merge_many(std::iter::empty::<ValidationResult<i32, String>>());
        // Same buggy shape: Some(empty_vec) instead of None.
        assert_eq!(merged.data, Some(vec![]));
        assert!(merged.errors.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn test_merge_many_all_inputs_no_data_returns_some_empty() {
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_error("e2".to_string());

        let merged = ValidationResult::merge_many(vec![r1, r2]);
        assert_eq!(merged.data, Some(vec![]));
        assert_eq!(merged.errors, vec!["e1".to_string(), "e2".to_string()]);
    }

    // -- flatten_strict() (issue #2867 fix) --
    // PROTOCOL_VERSION_12+ uses these. They restore the
    // `data.is_none() ⇔ no work done` invariant.

    #[test]
    fn test_flatten_strict_merges_non_empty_data() {
        let r1: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![1, 2]);
        let r2: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_data_and_errors(vec![3], vec!["e".to_string()]);
        let r3: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e2".to_string());

        let flat = ValidationResult::flatten_strict(vec![r1, r2, r3]);
        assert_eq!(flat.data, Some(vec![1, 2, 3]));
        assert_eq!(flat.errors, vec!["e".to_string(), "e2".to_string()]);
    }

    #[test]
    fn test_flatten_strict_empty_input_returns_none_data() {
        let flat: ValidationResult<Vec<i32>, String> =
            ValidationResult::flatten_strict(std::iter::empty());
        assert_eq!(flat.data, None);
        assert!(flat.errors.is_empty());
    }

    #[test]
    fn test_flatten_strict_all_inputs_no_data_returns_none() {
        // The whole point of strict: when no input contributed data,
        // return data:None — not Some(empty_vec). Downstream code
        // (process_validation_result_v0:241) keys on data.is_none().
        let r1: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<Vec<i32>, String> =
            ValidationResult::new_with_error("e2".to_string());

        let flat = ValidationResult::flatten_strict(vec![r1, r2]);
        assert!(flat.data.is_none());
        assert_eq!(flat.errors, vec!["e1".to_string(), "e2".to_string()]);
    }

    #[test]
    fn test_flatten_strict_some_empty_some_non_empty_returns_some() {
        // Mixed input: one had data:Some(empty_vec), another had
        // Some(non_empty). The aggregate is non-empty, so data:Some(...).
        let r1: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![]);
        let r2: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![42]);

        let flat = ValidationResult::flatten_strict(vec![r1, r2]);
        assert_eq!(flat.data, Some(vec![42]));
        assert!(flat.errors.is_empty());
    }

    #[test]
    fn test_flatten_strict_all_some_empty_returns_none() {
        // All inputs had data:Some(empty_vec). The aggregate Vec is
        // empty → data:None per the strict contract.
        let r1: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![]);
        let r2: ValidationResult<Vec<i32>, String> = ValidationResult::new_with_data(vec![]);

        let flat = ValidationResult::flatten_strict(vec![r1, r2]);
        assert!(flat.data.is_none());
        assert!(flat.errors.is_empty());
    }

    // -- merge_many_strict() (issue #2867 fix) --

    #[test]
    fn test_merge_many_strict_collects_non_empty_data() {
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_data(1);
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_data(2);
        let r3: ValidationResult<i32, String> = ValidationResult::new_with_error("e".to_string());

        let merged = ValidationResult::merge_many_strict(vec![r1, r2, r3]);
        assert_eq!(merged.data, Some(vec![1, 2]));
        assert_eq!(merged.errors, vec!["e".to_string()]);
    }

    #[test]
    fn test_merge_many_strict_empty_input_returns_none_data() {
        let merged: ValidationResult<Vec<i32>, String> = ValidationResult::merge_many_strict(
            std::iter::empty::<ValidationResult<i32, String>>(),
        );
        assert!(merged.data.is_none());
        assert!(merged.errors.is_empty());
    }

    #[test]
    fn test_merge_many_strict_all_inputs_no_data_returns_none() {
        // The bug-fixing case: all per-transition results returned
        // errors-only with no action. Strict aggregator surfaces this
        // as data:None so the downstream paid/unpaid switch picks unpaid.
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_error("e2".to_string());

        let merged = ValidationResult::merge_many_strict(vec![r1, r2]);
        assert!(merged.data.is_none());
        assert_eq!(merged.errors, vec!["e1".to_string(), "e2".to_string()]);
    }

    #[test]
    fn test_merge_many_strict_some_data_returns_some() {
        let r1: ValidationResult<i32, String> = ValidationResult::new_with_error("e1".to_string());
        let r2: ValidationResult<i32, String> = ValidationResult::new_with_data(7);

        let merged = ValidationResult::merge_many_strict(vec![r1, r2]);
        assert_eq!(merged.data, Some(vec![7]));
        assert_eq!(merged.errors, vec!["e1".to_string()]);
    }

    // -- merge_many_errors() --

    #[test]
    fn test_merge_many_errors_collects_all_errors() {
        let r1: SimpleValidationResult<String> =
            SimpleValidationResult::new_with_errors(vec!["a".to_string()]);
        let r2: SimpleValidationResult<String> =
            SimpleValidationResult::new_with_errors(vec!["b".to_string(), "c".to_string()]);
        let r3: SimpleValidationResult<String> = SimpleValidationResult::new();

        let merged = SimpleValidationResult::merge_many_errors(vec![r1, r2, r3]);
        assert_eq!(
            merged.errors,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_merge_many_errors_empty_input() {
        let merged: SimpleValidationResult<String> =
            SimpleValidationResult::merge_many_errors(std::iter::empty());
        assert!(merged.errors.is_empty());
    }

    // -- Default --

    #[test]
    fn test_default_is_empty() {
        let result: ValidationResult<i32, String> = ValidationResult::default();
        assert!(result.errors.is_empty());
        assert!(result.data.is_none());
    }

    // -- add_error / add_errors / merge --

    #[test]
    fn test_add_error() {
        let mut result: ValidationResult<i32, String> = ValidationResult::new();
        result.add_error("e1".to_string());
        result.add_error("e2".to_string());
        assert_eq!(result.errors, vec!["e1".to_string(), "e2".to_string()]);
    }

    #[test]
    fn test_add_errors() {
        let mut result: ValidationResult<i32, String> =
            ValidationResult::new_with_error("e1".to_string());
        result.add_errors(vec!["e2".to_string(), "e3".to_string()]);
        assert_eq!(result.errors.len(), 3);
    }

    #[test]
    fn test_merge_appends_errors_from_other() {
        let mut r1: ValidationResult<i32, String> =
            ValidationResult::new_with_error("a".to_string());
        let r2: ValidationResult<String, String> =
            ValidationResult::new_with_error("b".to_string());
        r1.merge(r2);
        assert_eq!(r1.errors, vec!["a".to_string(), "b".to_string()]);
    }

    // -- get_error / has_data / is_valid_with_data / set_data --

    #[test]
    fn test_get_error() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_errors(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(result.get_error(0), Some(&"a".to_string()));
        assert_eq!(result.get_error(1), Some(&"b".to_string()));
        assert_eq!(result.get_error(2), None);
    }

    #[test]
    fn test_has_data() {
        let with: ValidationResult<i32, String> = ValidationResult::new_with_data(1);
        let without: ValidationResult<i32, String> = ValidationResult::new();
        assert!(with.has_data());
        assert!(!without.has_data());
    }

    #[test]
    fn test_is_valid_with_data() {
        let valid_with_data: ValidationResult<i32, String> = ValidationResult::new_with_data(1);
        let valid_no_data: ValidationResult<i32, String> = ValidationResult::new();
        let invalid_with_data: ValidationResult<i32, String> =
            ValidationResult::new_with_data_and_errors(1, vec!["e".to_string()]);
        assert!(valid_with_data.is_valid_with_data());
        assert!(!valid_no_data.is_valid_with_data());
        assert!(!invalid_with_data.is_valid_with_data());
    }

    #[test]
    fn test_set_data() {
        let mut result: ValidationResult<i32, String> = ValidationResult::new();
        assert!(result.data.is_none());
        result.set_data(99);
        assert_eq!(result.data, Some(99));
    }

    #[test]
    fn test_into_result_without_data() {
        let result: ValidationResult<i32, String> =
            ValidationResult::new_with_data_and_errors(42, vec!["e".to_string()]);
        let without_data = result.into_result_without_data();
        assert!(without_data.data.is_none());
        assert_eq!(without_data.errors, vec!["e".to_string()]);
    }

    #[test]
    fn test_data_as_borrowed() {
        let result: ValidationResult<i32, String> = ValidationResult::new_with_data(42);
        assert_eq!(result.data_as_borrowed().unwrap(), &42);
    }

    #[test]
    fn test_data_as_borrowed_no_data() {
        let result: ValidationResult<i32, String> = ValidationResult::new();
        assert!(result.data_as_borrowed().is_err());
    }
}
