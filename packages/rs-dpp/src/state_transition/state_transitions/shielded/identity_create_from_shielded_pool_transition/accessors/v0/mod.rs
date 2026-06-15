use crate::address_funds::PlatformAddress;
use crate::shielded::SerializedAction;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_value::Identifier;

pub trait IdentityCreateFromShieldedPoolTransitionAccessorsV0 {
    /// Get the serialized Orchard actions (spend/output pairs).
    fn actions(&self) -> &[SerializedAction];
    /// Replace the serialized Orchard actions.
    fn set_actions(&mut self, actions: Vec<SerializedAction>);

    /// Get the public keys of the new identity.
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation];
    /// Replace the public keys of the new identity.
    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>);

    /// Get the fixed exit denomination (in credits).
    fn denomination(&self) -> u64;
    /// Set the fixed exit denomination (in credits).
    fn set_denomination(&mut self, denomination: u64);

    /// Get the Orchard anchor (Sinsemilla root of the note commitment tree).
    fn anchor(&self) -> [u8; 32];
    /// Set the Orchard anchor.
    fn set_anchor(&mut self, anchor: [u8; 32]);

    /// Get the Halo2 proof bytes.
    fn proof(&self) -> &[u8];
    /// Set the Halo2 proof bytes.
    fn set_proof(&mut self, proof: Vec<u8>);

    /// Get the RedPallas binding signature.
    fn binding_signature(&self) -> [u8; 64];
    /// Set the RedPallas binding signature.
    fn set_binding_signature(&mut self, binding_signature: [u8; 64]);

    /// Get the fallback address credited (minus penalty) if identity creation fails a stateful check.
    fn send_to_address_on_creation_failure(&self) -> &PlatformAddress;
    /// Set the fallback address credited (minus penalty) if identity creation fails a stateful check.
    fn set_send_to_address_on_creation_failure(
        &mut self,
        send_to_address_on_creation_failure: PlatformAddress,
    );

    /// Get the id of the new identity (derived from the spend nullifiers).
    fn identity_id(&self) -> Identifier;
    /// Set the id of the new identity.
    fn set_identity_id(&mut self, identity_id: Identifier);

    /// Extract nullifier bytes from each action.
    /// Generic over the element type: use `Vec<u8>` or `[u8; 32]` as needed.
    fn nullifiers<T: From<[u8; 32]>>(&self) -> Vec<T> {
        self.actions()
            .iter()
            .map(|a| T::from(a.nullifier))
            .collect()
    }
}
