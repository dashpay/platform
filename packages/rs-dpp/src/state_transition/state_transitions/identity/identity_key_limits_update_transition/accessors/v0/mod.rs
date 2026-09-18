use crate::fee::Credits;
use crate::identity::{KeyID, TimestampMillis};
use crate::prelude::{Identifier, IdentityNonce};

pub trait IdentityKeyLimitsUpdateTransitionAccessorsV0 {
    fn set_identity_id(&mut self, id: Identifier);
    fn identity_id(&self) -> Identifier;
    fn set_nonce(&mut self, nonce: IdentityNonce);
    fn nonce(&self) -> IdentityNonce;
    fn set_key_id(&mut self, key_id: KeyID);
    /// The key whose limits are raised
    fn key_id(&self) -> KeyID;
    fn set_total_budget(&mut self, total_budget: Option<Credits>);
    /// The new total budget of the key, `None` to leave it unchanged
    fn total_budget(&self) -> Option<Credits>;
    fn set_expires_at(&mut self, expires_at: Option<TimestampMillis>);
    /// The new expiry of the key, `None` to leave it unchanged
    fn expires_at(&self) -> Option<TimestampMillis>;
}
