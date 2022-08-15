use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Identity ${identity_id:?} already exists")]
pub struct IdentityAlreadyExistsError {
    identity_id: [u8; 32],
}

impl IdentityAlreadyExistsError {
    pub fn new(identity_id: [u8; 32]) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> &[u8; 32] {
        &self.identity_id
    }
}
