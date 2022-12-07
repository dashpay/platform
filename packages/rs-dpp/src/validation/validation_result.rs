use crate::errors::consensus::ConsensusError;

pub type SimpleValidationResult = ValidationResult<()>;

#[derive(Default, Debug)]
pub struct ValidationResult<TData: Clone> {
    pub errors: Vec<ConsensusError>,
    // TODO: data can be anything, figure out what to do with it
    data: Option<TData>,
}

impl<TData: Clone> ValidationResult<TData> {
    pub fn new(errors: Option<Vec<ConsensusError>>) -> Self {
        Self {
            errors: errors.unwrap_or_default(),
            data: None,
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
        self.errors.append(other.errors_mut());
    }

    pub fn errors(&self) -> &Vec<ConsensusError> {
        &self.errors
    }

    pub fn errors_mut(&mut self) -> &mut Vec<ConsensusError> {
        &mut self.errors
    }

    pub fn is_valid(&self) -> bool {
        self.errors().is_empty()
    }

    pub fn first_error(&self) -> Option<&ConsensusError> {
        self.errors.first()
    }

    pub fn set_data(&mut self, data: TData) {
        self.data = Some(data);
    }

    pub fn data(&self) -> Option<&TData> {
        self.data.as_ref()
    }

    pub fn into_result_without_data(self) -> ValidationResult<()> {
        ValidationResult {
            errors: self.errors,
            data: Some(()),
        }
    }
}
