use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;

pub type SimpleValidationResult = ValidationResult<()>;

#[derive(Default, Debug)]
pub struct ValidationResult<TData: Clone> {
    pub errors: Vec<ConsensusError>,
    data: Option<TData>,
}

impl<TData: Clone> ValidationResult<TData> {
    pub fn new_with_data(data: TData) -> Self {
        Self {
            errors: vec![],
            data: Some(data),
        }
    }
    pub fn new_with_errors(errors: Vec<ConsensusError>) -> Self {
        Self { errors, data: None }
    }

    pub fn map<F, U: Clone>(self, f: F) -> ValidationResult<U>
    where
        F: FnOnce(TData) -> U,
    {
        ValidationResult {
            errors: self.errors,
            data: self.data.map(f),
        }
    }

    pub fn add_error<T>(&mut self, error: T)
    where
        T: Into<ConsensusError>,
    {
        self.errors.push(error.into())
    }

    pub fn add_errors(&mut self, mut errors: Vec<ConsensusError>) {
        self.errors.append(&mut errors)
    }

    pub fn merge<TOtherData: Clone>(&mut self, mut other: ValidationResult<TOtherData>) {
        self.errors.append(&mut other.errors);
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn first_error(&self) -> Option<&ConsensusError> {
        self.errors.first()
    }

    pub fn into_result_without_data(self) -> ValidationResult<()> {
        ValidationResult {
            errors: self.errors,
            data: None,
        }
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

    pub fn data(&self) -> Result<&TData, ProtocolError> {
        self.data
            .as_ref()
            .ok_or(ProtocolError::CorruptedCodeExecution(format!(
                "trying to get validation result as data (errors are {:?})",
                self.errors
            )))
    }
}

impl<TData: Clone> From<TData> for ValidationResult<TData> {
    fn from(value: TData) -> Self {
        ValidationResult::new_with_data(value)
    }
}

impl<TData: Clone> From<Result<TData, ConsensusError>> for ValidationResult<TData> {
    fn from(value: Result<TData, ConsensusError>) -> Self {
        match value {
            Ok(data) => ValidationResult::new_with_data(data),
            Err(e) => ValidationResult::new_with_errors(vec![e]),
        }
    }
}
