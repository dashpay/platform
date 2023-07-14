#[cfg(feature = "state-transition-transformers")]
mod transformer;

use crate::identifier::Identifier;
use crate::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use crate::prelude::Revision;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityUpdateTransitionActionV0 {
    pub add_public_keys: Vec<IdentityPublicKey>,
    pub disable_public_keys: Vec<KeyID>,
    pub public_keys_disabled_at: Option<TimestampMillis>,
    pub identity_id: Identifier,
    pub revision: Revision,
}
