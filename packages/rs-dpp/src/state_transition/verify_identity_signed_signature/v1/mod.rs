use crate::consensus::signature::{InvalidStateTransitionSignatureError, SignatureError};
use crate::consensus::ConsensusError;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::IdentityPublicKey;
use crate::state_transition::StateTransition;
use crate::{BlsModule, ProtocolError};

impl StateTransition {
    /// Runs the checks of generation 0 and also refuses a signature they report does not
    /// verify, which today only a BLS12_381 key can produce.
    #[inline(always)]
    pub(super) fn verify_identity_signed_signature_v1(
        &self,
        public_key: &IdentityPublicKey,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        if self.identity_signed_signature_verdict_v0(public_key, bls)? {
            Ok(())
        } else {
            Err(ProtocolError::from(ConsensusError::SignatureError(
                SignatureError::InvalidStateTransitionSignatureError(
                    InvalidStateTransitionSignatureError::new(format!(
                        "{} signature does not verify",
                        public_key.key_type()
                    )),
                ),
            )))
        }
    }
}
