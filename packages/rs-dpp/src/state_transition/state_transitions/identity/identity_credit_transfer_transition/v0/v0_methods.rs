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
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::GetDataContractSecurityLevelRequirementFn;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

impl IdentityCreditTransferTransitionMethodsV0 for IdentityCreditTransferTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer<IdentityPublicKey>>(
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
        let mut transition: StateTransition = IdentityCreditTransferTransitionV0 {
            identity_id: identity.id(),
            recipient_id: to_identity_with_identifier,
            amount,
            nonce,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

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

        transition.sign_external(
            identity_public_key,
            &signer,
            None::<GetDataContractSecurityLevelRequirementFn>,
        )?;

        Ok(transition)
    }
}
