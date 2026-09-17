mod transformer;

use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::{KeyID, TimestampMillis};
use dpp::prelude::{IdentityNonce, Revision, UserFeeIncrease};

/// action v0
#[derive(Default, Debug, Clone)]
pub struct IdentityKeyLimitsUpdateTransitionActionV0 {
    /// identity id
    pub identity_id: Identifier,
    /// revision
    pub revision: Revision,
    /// nonce used to prevent replay attacks
    pub nonce: IdentityNonce,
    /// the key whose limits are raised
    pub key_id: KeyID,
    /// the new total budget of the key, `None` when it stays as it is
    pub total_budget: Option<Credits>,
    /// the new expiry of the key, `None` when it stays as it is
    pub expires_at: Option<TimestampMillis>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
