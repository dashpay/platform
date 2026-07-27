use crate::state_transition_action::shielded::identity_create_from_shielded_pool::v0::IdentityCreateFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use std::collections::BTreeMap;

impl IdentityCreateFromShieldedPoolTransitionActionV0 {
    /// Transforms the identity-create-from-shielded-pool transition into an action, building the new
    /// identity (id + keys + balance = `denomination`) from the transition payload.
    pub fn try_from_transition(
        value: &IdentityCreateFromShieldedPoolTransitionV0,
        current_total_balance: Credits,
        fee_amount: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let public_keys: BTreeMap<KeyID, IdentityPublicKey> = value
            .public_keys
            .iter()
            .map(|key| {
                let public_key: IdentityPublicKey = key.into();
                (public_key.id(), public_key)
            })
            .collect();

        // Re-derive the id from the spend nullifiers (the canonical value). The wire `identity_id`
        // is advisory; consensus always uses the derived value so a malformed/malicious wire id
        // cannot redirect the created identity (the Orchard sighash also binds the derived id).
        let identity_id = dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions(&value.actions);
        let mut identity =
            Identity::new_with_id_and_keys(identity_id, public_keys, platform_version)?;
        // The identity is created holding the FULL denomination. The fee is moved out of this
        // balance into the fee pools at execution (so the credit supply is conserved).
        identity.set_balance(value.denomination);

        let notes: Vec<ShieldedActionNote> =
            value.actions.iter().map(ShieldedActionNote::from).collect();

        Ok(IdentityCreateFromShieldedPoolTransitionActionV0 {
            identity,
            notes,
            anchor: value.anchor,
            denomination: value.denomination,
            fee_amount,
            current_total_balance,
        })
    }
}
