use crate::errors::consensus::ConsensusError;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use std::fmt::Debug;

mod flatten;
mod merge_many;

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
    /// Aggregate a list of `ValidationResult<Vec<TData>, E>` into a single
    /// result. Dispatches to the version selected by `platform_version`:
    ///
    /// - **v0** (`PROTOCOL_VERSION_11` and below): always returns
    ///   `data: Some(Vec<...>)`, including `Some(empty_vec)` when no input
    ///   contributed any data. Preserved for chain reproducibility.
    /// - **v1** (`PROTOCOL_VERSION_12`+): returns `data: None` when no input
    ///   contributed any data. Honors the invariant
    ///   `data.is_none() ⇔ no work done`, which downstream code (e.g.
    ///   `process_validation_result_v0:241`) relies on to choose between
    ///   `PaidConsensusError` and `UnpaidConsensusError`.
    ///
    /// # v1 caller-intent ambiguity
    ///
    /// v1 keys on `aggregate_data.is_empty()` to decide between
    /// `data: None` and `data: Some(_)`, which collapses two distinct
    /// caller intents into the same output: every input had `data: None`
    /// (truly no work) and every input had `data: Some(empty_vec)`
    /// (validated but produced no output). v1 cannot distinguish those
    /// at the aggregate level — both yield `data: None` and are routed
    /// to `UnpaidConsensusError` downstream. Callers that need "validated
    /// but no actions" must signal that with at least one non-empty entry.
    ///
    /// See issue #2867 for context on the v0 → v1 change.
    pub fn flatten<I: IntoIterator<Item = ValidationResult<Vec<TData>, E>>>(
        items: I,
        platform_version: &PlatformVersion,
    ) -> Result<ValidationResult<Vec<TData>, E>, ProtocolError> {
        match platform_version.dpp.validation.validation_result.flatten {
            0 => Ok(flatten::v0::flatten_v0(items)),
            1 => Ok(flatten::v1::flatten_v1(items)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ValidationResult::flatten".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl<TData: Clone, E: Debug> ValidationResult<TData, E> {
    /// Aggregate a list of `ValidationResult<TData, E>` into a
    /// `ValidationResult<Vec<TData>, E>`. Dispatches to the version selected
    /// by `platform_version`:
    ///
    /// - **v0** (`PROTOCOL_VERSION_11` and below): always returns
    ///   `data: Some(Vec<...>)`, including `Some(empty_vec)` when no input
    ///   contributed any data. Preserved for chain reproducibility.
    /// - **v1** (`PROTOCOL_VERSION_12`+): returns `data: None` when no input
    ///   contributed any data. See [`flatten`] for the invariant this
    ///   restores.
    ///
    /// Unlike [`flatten`], `merge_many` operates on per-item `TData` (not
    /// `Vec<TData>`), so each `Some(_)` input contributes exactly one
    /// element — there is no `Some(empty_vec)`-input collapse hazard at
    /// this layer.
    ///
    /// See issue #2867 for context on the v0 → v1 change.
    ///
    /// [`flatten`]: ValidationResult::flatten
    pub fn merge_many<I: IntoIterator<Item = ValidationResult<TData, E>>>(
        items: I,
        platform_version: &PlatformVersion,
    ) -> Result<ValidationResult<Vec<TData>, E>, ProtocolError> {
        match platform_version.dpp.validation.validation_result.merge_many {
            0 => Ok(merge_many::v0::merge_many_v0(items)),
            1 => Ok(merge_many::v1::merge_many_v1(items)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ValidationResult::merge_many".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
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

    // -- facade dispatch (flatten / merge_many take platform_version) --
    //
    // These verify the version field on PlatformVersion correctly steers the
    // facade to v0 vs v1 semantics. Per-version behavior is tested in each
    // version's own module (e.g. `flatten::v1::tests`).

    #[test]
    fn test_facade_flatten_v0_returns_some_empty_on_no_data() {
        // PROTOCOL_VERSION_11 maps to dpp.validation.validation_result.flatten = 0
        let pv = PlatformVersion::get(11).expect("v11 exists");
        let r1: ValidationResult<Vec<i32>, ConsensusError> =
            ValidationResult::new_with_errors(vec![]);
        let flat = ValidationResult::flatten(vec![r1], pv).expect("dispatch ok");
        assert_eq!(flat.data, Some(vec![]));
    }

    #[test]
    fn test_facade_flatten_v1_returns_none_on_no_data() {
        // PROTOCOL_VERSION_12 maps to dpp.validation.validation_result.flatten = 1
        let pv = PlatformVersion::get(12).expect("v12 exists");
        let r1: ValidationResult<Vec<i32>, ConsensusError> =
            ValidationResult::new_with_errors(vec![]);
        let flat = ValidationResult::flatten(vec![r1], pv).expect("dispatch ok");
        assert!(flat.data.is_none());
    }

    #[test]
    fn test_facade_merge_many_v0_returns_some_empty_on_no_data() {
        let pv = PlatformVersion::get(11).expect("v11 exists");
        let r1: ValidationResult<i32, ConsensusError> = ValidationResult::new_with_errors(vec![]);
        let merged = ValidationResult::merge_many(vec![r1], pv).expect("dispatch ok");
        assert_eq!(merged.data, Some(vec![]));
    }

    #[test]
    fn test_facade_merge_many_v1_returns_none_on_no_data() {
        let pv = PlatformVersion::get(12).expect("v12 exists");
        let r1: ValidationResult<i32, ConsensusError> = ValidationResult::new_with_errors(vec![]);
        let merged = ValidationResult::merge_many(vec![r1], pv).expect("dispatch ok");
        assert!(merged.data.is_none());
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
