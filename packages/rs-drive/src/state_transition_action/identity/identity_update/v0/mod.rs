mod transformer;

use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::prelude::Revision;
use serde::{Deserialize, Serialize};

/// action v0
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityUpdateTransitionActionV0 {
    /// add public keys
    pub add_public_keys: Vec<IdentityPublicKey>,
    /// disable public keys
    pub disable_public_keys: Vec<KeyID>,
    /// public keys disabled at
    pub public_keys_disabled_at: Option<TimestampMillis>,
    /// identity id
    pub identity_id: Identifier,
    /// revision
    pub revision: Revision,
}
