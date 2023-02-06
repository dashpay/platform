use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::identifier::Identifier;

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

impl From<IdentityNotPresentError> for ConsensusError {
    fn from(err: IdentityNotPresentError) -> Self {
        Self::IdentityNotPresentError(err)
    }
}
