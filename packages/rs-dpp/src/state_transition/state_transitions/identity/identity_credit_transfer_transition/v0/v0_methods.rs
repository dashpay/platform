#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
use crate::consensus::basic::identity::{
    IdentityCreditTransferToSelfError, InvalidIdentityCreditTransferAmountError,
};
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
use crate::validation::SimpleConsensusValidationResult;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, signer::Signer, Identity,
        IdentityPublicKey, KeyType, Purpose, SecurityLevel,
    },
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;

use crate::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
use crate::state_transition::identity_credit_transfer_transition::MIN_TRANSFER_AMOUNT;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{
    consensus_errors_as_protocol_error, GetDataContractSecurityLevelRequirementFn,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

#[cfg(any(
    feature = "state-transition-validation",
    feature = "state-transition-signing"
))]
impl IdentityCreditTransferTransitionV0 {
    /// Shared single source of truth for the v0 basic-structure rules of an
    /// identity credit transfer: rejects self-transfers and amounts below
    /// `MIN_TRANSFER_AMOUNT`. Used by both the client-side SDK constructor
    /// (to fail fast before any signing work) and drive-abci's structure
    /// validator (to enforce the same rules server-side), so the two cannot
    /// drift.
    pub fn validate_basic_structure_v0(&self) -> SimpleConsensusValidationResult {
        if self.identity_id == self.recipient_id {
            return SimpleConsensusValidationResult::new_with_error(
                IdentityCreditTransferToSelfError::default().into(),
            );
        }

        if self.amount < MIN_TRANSFER_AMOUNT {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidIdentityCreditTransferAmountError::new(self.amount, MIN_TRANSFER_AMOUNT)
                    .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

impl IdentityCreditTransferTransitionMethodsV0 for IdentityCreditTransferTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        to_identity_with_identifier: Identifier,
        amount: u64,
        user_fee_increase: UserFeeIncrease,
        signer: S,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        nonce: IdentityNonce,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let transition_v0 = IdentityCreditTransferTransitionV0 {
            identity_id: identity.id(),
            recipient_id: to_identity_with_identifier,
            amount,
            nonce,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        };

        // Pre-signing structure check that mirrors drive-abci's
        // `IdentityCreditTransferStateTransitionStructureValidationV0`. This
        // catches self-transfers and below-minimum amounts before any async
        // signer work is performed.
        //
        // LOCKSTEP: hard-coded to the v0 basic-structure check. If a future v1
        // basic-structure is introduced for this transition, both the
        // drive-abci server dispatcher AND this SDK constructor must be
        // updated together.
        let pre_validation_result = transition_v0.validate_basic_structure_v0();
        if let Some(error) = consensus_errors_as_protocol_error(pre_validation_result) {
            return Err(error);
        }

        let mut transition: StateTransition = transition_v0.into();

        let identity_public_key = match signing_withdrawal_key_to_use {
            Some(key) => {
                if signer.can_sign_with(key) {
                    key
                } else {
                    tracing::error!(
                        key_id = key.id(),
                        "specified transfer key cannot be used for signing"
                    );
                    return Err(
                        ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                            "specified transfer public key cannot be used for signing".to_string(),
                        ),
                    );
                }
            }
            None => {
                let key_result = identity.get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                );

                key_result.ok_or_else(|| {
                    tracing::error!(
                        identity_id = %identity.id(),
                        total_keys = identity.public_keys().len(),
                        "no transfer public key found in identity"
                    );
                    for (key_id, key) in identity.public_keys() {
                        tracing::debug!(key_id, purpose = ?key.purpose(), "available key");
                    }
                    ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                        "no transfer public key".to_string(),
                    )
                })?
            }
        };

        transition
            .sign_external(
                identity_public_key,
                &signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )
            .await?;

        Ok(transition)
    }
}
