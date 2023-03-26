use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;

pub type SimpleValidationResult = ValidationResult<()>;

#[derive(Default, Debug)]
pub struct ValidationResult<TData: Clone> {
    pub system_error: Option<ProtocolError>,
    pub consensus_errors: Vec<ConsensusError>,
    // TODO: data can be anything, figure out what to do with it
    data: Option<TData>,
}

impl<TData: Clone> ValidationResult<TData> {
    pub fn new(errors: Option<Vec<ConsensusError>>) -> Self {
        Self {
            system_error: None,
            consensus_errors: errors.unwrap_or_default(),
            data: None,
        }
    }

    pub fn map<F, U: Clone>(self, f: F) -> ValidationResult<U>
    where
        F: FnOnce(TData) -> U,
    {
        ValidationResult {
            system_error: None,
            consensus_errors: self.consensus_errors,
            data: self.data.map(f),
        }
    }

    pub fn add_error<T>(&mut self, error: T)
    where
        T: Into<ConsensusError>,
    {
        self.consensus_errors.push(error.into())
    }

    pub fn add_errors(&mut self, mut errors: Vec<ConsensusError>) {
        self.consensus_errors.append(&mut errors)
    }

    pub fn merge<TOtherData: Clone>(&mut self, mut other: ValidationResult<TOtherData>) {
        self.consensus_errors.append(other.errors_mut());
    }

    pub fn errors(&self) -> &Vec<ConsensusError> {
        &self.consensus_errors
    }

    pub fn errors_mut(&mut self) -> &mut Vec<ConsensusError> {
        &mut self.consensus_errors
    }

    pub fn is_valid(&self) -> bool {
        self.errors().is_empty()
    }

    pub fn first_error(&self) -> Option<&ConsensusError> {
        self.consensus_errors.first()
    }

    pub fn set_data(&mut self, data: TData) {
        self.data = Some(data);
    }

    pub fn data(&self) -> Option<&TData> {
        self.data.as_ref()
    }

    pub fn take_data(&mut self) -> Option<TData> {
        self.data.take()
    }

    pub fn into_result_without_data(self) -> ValidationResult<()> {
        ValidationResult {
            system_error: None,
            consensus_errors: self.consensus_errors,
            data: Some(()),
        }
    }
}

impl From<ProtocolError> for ValidationResult<()> {
    fn from(value: ProtocolError) -> Self {
        ValidationResult {
            system_error: Some(value),
            consensus_errors: vec![],
            data: None,
        }
    }
}

impl From<ConsensusError> for ValidationResult<()> {
    fn from(value: ConsensusError) -> Self {
        ValidationResult {
            system_error: None,
            consensus_errors: vec![value],
            data: None,
        }
    }
}

impl From<anyhow::Error> for ValidationResult<()> {
    fn from(value: anyhow::Error) -> Self {
        ValidationResult {
            system_error: value.into(),
            consensus_errors: vec![],
            data: None,
        }
    }
}
