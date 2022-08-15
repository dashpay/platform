use crate::errors::StateError;

#[derive(Debug, Default)]
pub struct DataTriggerExecutionResult {
    pub errors: Vec<StateError>,
}

impl From<Vec<StateError>> for DataTriggerExecutionResult {
    fn from(errors: Vec<StateError>) -> Self {
        DataTriggerExecutionResult { errors }
    }
}

impl DataTriggerExecutionResult {
    /// Add an error to result
    pub fn add_error(&mut self, err: StateError) {
        self.errors.push(err)
    }

    /// Get all Trigger Execution Errors
    pub fn get_errors(&self) -> &Vec<StateError> {
        &self.errors
    }

    /// Check if result has no errors
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}
