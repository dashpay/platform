use platform_value::Value;
use std::sync::Mutex;

use crate::identity::validation::TPublicKeysValidator;
use crate::validation::SimpleValidationResult;
use crate::NonConsensusError;

#[cfg(feature = "fixtures-and-mocks")]
pub struct PublicKeysValidatorMock {
    returns: Mutex<Result<SimpleValidationResult, NonConsensusError>>,
    returns_fn:
        Mutex<Option<Box<dyn Fn() -> Result<SimpleValidationResult, NonConsensusError> + 'static>>>,
    called_with: Mutex<Vec<Value>>,
}

impl Default for PublicKeysValidatorMock {
    fn default() -> Self {
        Self::new()
    }
}

impl PublicKeysValidatorMock {
    pub fn new() -> Self {
        Self {
            returns: Mutex::new(Ok(SimpleValidationResult::default())),
            returns_fn: Mutex::new(None),
            called_with: Mutex::new(vec![]),
        }
    }

    pub fn returns(&self, result: Result<SimpleValidationResult, NonConsensusError>) {
        *self.returns.lock().unwrap() = result;
    }

    pub fn returns_fun(
        &self,
        func: impl Fn() -> Result<SimpleValidationResult, NonConsensusError> + 'static,
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
    ) -> Result<SimpleValidationResult, NonConsensusError> {
        *self.called_with.lock().unwrap() = Vec::from(raw_public_keys);
        let guard = self.returns_fn.lock().unwrap();
        let fun = guard.as_ref().unwrap();

        fun()
        // self.returns.lock().unwrap().clone()
    }
}
