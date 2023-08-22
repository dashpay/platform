use crate::prelude::AssetLockProof;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::NonConsensusError;
use platform_value::Identifier;

pub trait IdentityCreateTransitionAccessorsV0 {
    /// Set asset lock
    fn set_asset_lock_proof(
        &mut self,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(), NonConsensusError>;
    /// Get asset lock proof
    fn asset_lock_proof(&self) -> &AssetLockProof;
    /// Get identity public keys
    fn public_keys(&self) -> &[IdentityPublicKeyInCreation];
    /// Replaces existing set of public keys with a new one
    fn set_public_keys(&mut self, public_keys: Vec<IdentityPublicKeyInCreation>);
    /// Adds public keys to the existing public keys array
    fn add_public_keys(&mut self, public_keys: &mut Vec<IdentityPublicKeyInCreation>);
    /// Returns identity id
    fn identity_id(&self) -> Identifier;
    /// Returns Owner ID
    fn owner_id(&self) -> Identifier;
}
