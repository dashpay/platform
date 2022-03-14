use crate::errors::consensus::AbstractConsensusError;

pub struct ValidationResult {
    errors: Vec<AbstractConsensusError>,
    // TODO: data can be anything, figure out what to do with it
    data: Option<AbstractConsensusError>
}

impl ValidationResult {
    pub fn new(errors: Option<Vec<AbstractConsensusError>>) -> Self {
        Self {
            errors: errors.unwrap_or_else(|| Vec::new()),
            data: None
        }
    }
}