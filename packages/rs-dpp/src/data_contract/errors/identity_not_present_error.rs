use platform_value::Identifier;
use thiserror::Error;

use crate::ProtocolError;

// @append_only
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Identity is not present")]
pub struct IdentityNotPresentError {
    id: Identifier,
}

impl IdentityNotPresentError {
    pub fn new(id: Identifier) -> Self {
        Self { id }
    }

    pub fn id(&self) -> Identifier {
        self.id
    }
}

impl From<IdentityNotPresentError> for ProtocolError {
    fn from(err: IdentityNotPresentError) -> Self {
        Self::IdentityNotPresentError(err)
    }
}
