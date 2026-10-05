use crate::consensus::signature::{InvalidStateTransitionSignatureError, SignatureError};
use crate::consensus::ConsensusError;
use crate::identity::IdentityPublicKey;
use crate::state_transition::StateTransition;
use crate::{BlsModule, ProtocolError};

impl StateTransition {
    /// Runs the checks of generation 0 and also refuses a BLS12_381 signature that does not
    /// verify.
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
                    InvalidStateTransitionSignatureError::new(
                        "bls signature does not verify".to_string(),
                    ),
                ),
            )))
        }
    }
}
