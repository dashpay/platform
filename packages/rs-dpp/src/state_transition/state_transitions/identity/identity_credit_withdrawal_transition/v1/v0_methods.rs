#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{
        accessors::IdentityGettersV0, core_script::CoreScript, signer::Signer, Identity,
        IdentityPublicKey, KeyType, Purpose, SecurityLevel,
    },
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::{
        identity_credit_withdrawal_transition::methods::PreferredKeyPurposeForSigningWithdrawal,
        GetDataContractSecurityLevelRequirementFn, StateTransition,
    },
    withdrawal::Pooling,
    ProtocolError,
};

#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

use crate::state_transition::identity_credit_withdrawal_transition::methods::IdentityCreditWithdrawalTransitionMethodsV0;
use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;

impl IdentityCreditWithdrawalTransitionMethodsV0 for IdentityCreditWithdrawalTransitionV1 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        output_script: Option<CoreScript>,
        amount: u64,
        pooling: Pooling,
        core_fee_per_byte: u32,
        user_fee_increase: UserFeeIncrease,
        signer: S,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        preferred_key_purpose_for_signing_withdrawal: PreferredKeyPurposeForSigningWithdrawal,
        nonce: IdentityNonce,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let mut transition: StateTransition = IdentityCreditWithdrawalTransitionV1 {
            identity_id: identity.id(),
            amount,
            core_fee_per_byte,
            pooling,
            output_script,
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
                    return Err(
                        ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                            "specified withdrawal public key cannot be used for signing"
                                .to_string(),
                        ),
                    );
                }
            }
            None => {
                let mut key: Option<&IdentityPublicKey>;

                match preferred_key_purpose_for_signing_withdrawal {
                    PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred => {
                        key = identity.get_first_public_key_matching(
                            Purpose::OWNER,
                            SecurityLevel::full_range().into(),
                            KeyType::all_key_types().into(),
                            true,
                        );

                        if key.is_none() || !signer.can_sign_with(key.unwrap()) {
                            key = identity.get_first_public_key_matching(
                                Purpose::TRANSFER,
                                SecurityLevel::full_range().into(),
                                KeyType::all_key_types().into(),
                                true,
                            );
                        }
                    }
                    PreferredKeyPurposeForSigningWithdrawal::TransferPreferred
                    | PreferredKeyPurposeForSigningWithdrawal::Any => {
                        key = identity.get_first_public_key_matching(
                            Purpose::TRANSFER,
                            SecurityLevel::full_range().into(),
                            KeyType::all_key_types().into(),
                            true,
                        );

                        if key.is_none() || !signer.can_sign_with(key.unwrap()) {
                            key = identity.get_first_public_key_matching(
                                Purpose::OWNER,
                                SecurityLevel::full_range().into(),
                                KeyType::all_key_types().into(),
                                true,
                            );
                        }
                    }
                    PreferredKeyPurposeForSigningWithdrawal::OwnerOnly => {
                        key = identity.get_first_public_key_matching(
                            Purpose::OWNER,
                            SecurityLevel::full_range().into(),
                            KeyType::all_key_types().into(),
                            true,
                        );
                    }
                    PreferredKeyPurposeForSigningWithdrawal::TransferOnly => {
                        key = identity.get_first_public_key_matching(
                            Purpose::TRANSFER,
                            SecurityLevel::full_range().into(),
                            KeyType::all_key_types().into(),
                            true,
                        );
                    }
                }

                key.ok_or_else(|| {
                    ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                        "no withdrawal public key".to_string(),
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
