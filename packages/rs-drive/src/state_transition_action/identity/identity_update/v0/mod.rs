mod transformer;

use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{IdentityNonce, Revision, UserFeeIncrease};
use serde::{Deserialize, Serialize};

/// action v0
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityUpdateTransitionActionV0 {
    /// add public keys
    pub add_public_keys: Vec<IdentityPublicKey>,
    /// disable public keys
    pub disable_public_keys: Vec<KeyID>,
    /// identity id
    pub identity_id: Identifier,
    /// revision
    pub revision: Revision,
    /// nonce used to prevent replay attacks
    pub nonce: IdentityNonce,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
