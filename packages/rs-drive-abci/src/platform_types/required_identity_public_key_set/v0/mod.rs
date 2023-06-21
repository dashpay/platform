use serde::{Deserialize, Serialize};

/// Required public key set for an identity
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequiredIdentityPublicKeysSet {
    /// Authentication key with master security level
    pub master: Vec<u8>,
    /// Authentication key with high security level
    pub high: Vec<u8>,
}
