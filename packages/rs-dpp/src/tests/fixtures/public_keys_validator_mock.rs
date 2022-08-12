use std::sync::Mutex;

use serde_json::Value;

use crate::identity::validation::TPublicKeysValidator;
use crate::validation::ValidationResult;
use crate::NonConsensusError;

#[cfg(test)]
pub struct PublicKeysValidatorMock {
    returns: Mutex<Result<ValidationResult<()>, NonConsensusError>>,
    returns_fn:
        Mutex<Option<Box<dyn Fn() -> Result<ValidationResult<()>, NonConsensusError> + 'static>>>,
    called_with: Mutex<Vec<Value>>,
}

impl PublicKeysValidatorMock {
    pub fn new() -> Self {
        Self {
            returns: Mutex::new(Ok(ValidationResult::default())),
            returns_fn: Mutex::new(None),
            called_with: Mutex::new(vec![]),
        }
    }

    pub fn returns(&self, result: Result<ValidationResult<()>, NonConsensusError>) {
        *self.returns.lock().unwrap() = result;
    }

    pub fn returns_fun(
        &self,
        func: impl Fn() -> Result<ValidationResult<()>, NonConsensusError> + 'static,
    ) {
        *self.returns_fn.lock().unwrap() = Some(Box::new(func))
    }

    pub fn called_with(&self) -> Vec<Value> {
        self.called_with.lock().unwrap().clone()
    }
}

impl TPublicKeysValidator for PublicKeysValidatorMock {
    fn validate_keys(
        &self,
        raw_public_keys: &[Value],
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        *self.called_with.lock().unwrap() = Vec::from(raw_public_keys);
        let guard = self.returns_fn.lock().unwrap();
        let fun = guard.as_ref().unwrap();
        let result = fun();
        result
        // self.returns.lock().unwrap().clone()
    }
}
