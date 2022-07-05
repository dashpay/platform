use crate::errors::consensus::ConsensusError;

#[derive(Default, Debug)]
pub struct ValidationResult {
    pub errors: Vec<ConsensusError>,
    // TODO: data can be anything, figure out what to do with it
    // data: Option<ConsensusError>
}

impl ValidationResult {
    pub fn new(errors: Option<Vec<ConsensusError>>) -> Self {
        Self {
            errors: errors.unwrap_or_default(),
            // data: None
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

    pub fn merge(&mut self, mut other: ValidationResult) {
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
}
