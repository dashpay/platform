use crate::identity::IdentityPublicKey;
use crate::identity::signer::Signer;
use crate::ProtocolError;
use crate::state_transition::StateTransitionLike;

pub(super) trait StateTransitionIdentitySignExternalV0
    where
        Self: StateTransitionLike,
{
    fn sign_external_v0<S: Signer>(
        &mut self,
        identity_public_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<(), ProtocolError> {
        self.verify_public_key_level_and_purpose(identity_public_key)?;
        self.verify_public_key_is_enabled(identity_public_key)?;
        let data = self.signable_bytes()?;
        self.set_signature(signer.sign(identity_public_key, data.as_slice())?);
        self.set_signature_public_key_id(identity_public_key.id);
        Ok(())
    }
}