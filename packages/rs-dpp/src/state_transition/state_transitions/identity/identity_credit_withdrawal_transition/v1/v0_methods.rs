#[cfg(feature = "state-transition-signing")]
use crate::{
    consensus::ConsensusError,
    identity::{
        accessors::IdentityGettersV0, core_script::CoreScript, signer::Signer, Identity,
        IdentityPublicKey, KeyType, Purpose, SecurityLevel,
    },
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::{
        identity_credit_withdrawal_transition::methods::{
            validate_withdrawal_output_script, PreferredKeyPurposeForSigningWithdrawal,
        },
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
    async fn try_from_identity<S: Signer<IdentityPublicKey>>(
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
        let identity_public_key = match signing_withdrawal_key_to_use {
            Some(key) => key,
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

        validate_withdrawal_output_script(identity_public_key, output_script.as_ref())
            .map_err(ConsensusError::from)?;

        if signing_withdrawal_key_to_use.is_some() && !signer.can_sign_with(identity_public_key) {
            return Err(
                ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                    "specified withdrawal public key cannot be used for signing".to_string(),
                ),
            );
        }

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

#[cfg(all(test, feature = "state-transition-signing"))]
mod tests {
    use super::*;
    use crate::address_funds::AddressWitness;
    use crate::consensus::basic::identity::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use platform_value::{BinaryData, Identifier};

    #[derive(Debug)]
    struct UnexpectedSigner {
        allow_availability_query: bool,
    }

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for UnexpectedSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            panic!("a refused withdrawal must not be signed");
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("identity credit withdrawals do not create an address witness");
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            assert!(
                self.allow_availability_query,
                "an explicit OWNER key must be refused before querying availability"
            );
            true
        }
    }

    #[tokio::test]
    async fn should_refuse_an_explicit_owner_key_before_querying_the_signer() {
        let platform_version = PlatformVersion::latest();
        let key = IdentityPublicKey::from(IdentityPublicKeyV0 {
            id: 17,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::CRITICAL,
            ..Default::default()
        });
        let identity = Identity::new_with_id_and_keys(
            Identifier::default(),
            [(17, key.clone())].into(),
            platform_version,
        )
        .expect("identity");
        let output_script = CoreScript::from_bytes(vec![0x51]);

        let error = IdentityCreditWithdrawalTransitionV1::try_from_identity(
            &identity,
            Some(output_script.clone()),
            1_000_000,
            Pooling::Never,
            1,
            0,
            UnexpectedSigner {
                allow_availability_query: false,
            },
            Some(&key),
            PreferredKeyPurposeForSigningWithdrawal::TransferPreferred,
            1,
            platform_version,
            None,
        )
        .await
        .expect_err("an explicit OWNER key cannot sign a withdrawal with an output script");

        assert_owner_script_refusal(error, &output_script);
    }

    #[tokio::test]
    async fn should_refuse_a_selected_owner_key_without_falling_back_to_transfer() {
        let platform_version = PlatformVersion::latest();
        let owner_key = IdentityPublicKey::from(IdentityPublicKeyV0 {
            id: 17,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::CRITICAL,
            ..Default::default()
        });
        let transfer_key = IdentityPublicKey::from(IdentityPublicKeyV0 {
            id: 18,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            ..Default::default()
        });
        let identity = Identity::new_with_id_and_keys(
            Identifier::default(),
            [(17, owner_key), (18, transfer_key)].into(),
            platform_version,
        )
        .expect("identity");
        let output_script = CoreScript::from_bytes(vec![0x51]);

        let error = IdentityCreditWithdrawalTransitionV1::try_from_identity(
            &identity,
            Some(output_script.clone()),
            1_000_000,
            Pooling::Never,
            1,
            0,
            UnexpectedSigner {
                allow_availability_query: true,
            },
            None,
            PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred,
            1,
            platform_version,
            None,
        )
        .await
        .expect_err("the selected OWNER key cannot sign a withdrawal with an output script");

        assert_owner_script_refusal(error, &output_script);
    }

    fn assert_owner_script_refusal(error: ProtocolError, output_script: &CoreScript) {
        let ProtocolError::ConsensusError(error) = error else {
            panic!("expected a consensus error, got {error:?}");
        };
        assert_eq!(
            *error,
            ConsensusError::from(
                WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError::new(
                    output_script.clone(),
                    17,
                ),
            ),
        );
    }
}
