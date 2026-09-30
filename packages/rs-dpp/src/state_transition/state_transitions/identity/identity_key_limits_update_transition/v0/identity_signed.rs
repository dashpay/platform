use crate::identity::SecurityLevel::{CRITICAL, MASTER};
use crate::identity::{KeyID, Purpose, SecurityLevel};
use crate::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use crate::state_transition::StateTransitionIdentitySigned;

impl StateTransitionIdentitySigned for IdentityKeyLimitsUpdateTransitionV0 {
    fn signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = key_id
    }

    /// A MASTER key, or a CRITICAL key. The CRITICAL key must carry no limits itself, which the
    /// identity signature validation checks against state.
    fn security_level_requirement(&self, _purpose: Purpose) -> Vec<SecurityLevel> {
        vec![MASTER, CRITICAL]
    }
}
