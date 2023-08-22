mod transformer;

use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::prelude::Revision;
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
