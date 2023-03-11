use platform_value::identifier::Identifier;
use thiserror::Error;

use crate::ProtocolError;

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
        self.id.clone()
    }
}

impl From<IdentityNotPresentError> for ProtocolError {
    fn from(err: IdentityNotPresentError) -> Self {
        Self::IdentityNotPresentError(err)
    }
}
