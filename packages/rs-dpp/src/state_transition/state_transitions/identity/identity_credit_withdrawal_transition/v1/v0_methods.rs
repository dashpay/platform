#[cfg(feature = "state-transition-signing")]
use crate::{
    consensus::ConsensusError,
    identity::{
        accessors::IdentityGettersV0, core_script::CoreScript,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, signer::Signer, Identity,
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
                // The first key of `purpose` the signer holds, else the first key of `purpose`.
                let held = |purpose: Purpose| {
                    identity
                        .public_keys()
                        .values()
                        .find(|key| key.purpose() == purpose && signer.can_sign_with(key))
                };
                let first = |purpose: Purpose| {
                    identity.get_first_public_key_matching(
                        purpose,
                        SecurityLevel::full_range().into(),
                        KeyType::all_key_types().into(),
                        true,
                    )
                };

                let key = match preferred_key_purpose_for_signing_withdrawal {
                    PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred
                        if output_script.is_none() =>
                    {
                        held(Purpose::OWNER)
                            .or_else(|| held(Purpose::TRANSFER))
                            .or_else(|| first(Purpose::TRANSFER))
                    }
                    // An OWNER key cannot sign a withdrawal to a chosen address, so there an
                    // OWNER preference tries TRANSFER first; OWNER stays the fallback and is
                    // refused below.
                    PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred
                    | PreferredKeyPurposeForSigningWithdrawal::TransferPreferred
                    | PreferredKeyPurposeForSigningWithdrawal::Any => held(Purpose::TRANSFER)
                        .or_else(|| held(Purpose::OWNER))
                        .or_else(|| first(Purpose::OWNER)),
                    PreferredKeyPurposeForSigningWithdrawal::OwnerOnly => {
                        held(Purpose::OWNER).or_else(|| first(Purpose::OWNER))
                    }
                    PreferredKeyPurposeForSigningWithdrawal::TransferOnly => {
                        held(Purpose::TRANSFER).or_else(|| first(Purpose::TRANSFER))
                    }
                };

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
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use crate::state_transition::StateTransitionIdentitySigned;
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

    /// A signer that holds every key and returns a fixed signature.
    #[derive(Debug)]
    struct SigningSigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for SigningSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Ok(BinaryData::new(vec![0; 65]))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("identity credit withdrawals do not create an address witness");
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
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

        // Through the enum constructor with no version, as the SDK calls it.
        let error = IdentityCreditWithdrawalTransition::try_from_identity(
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

    fn owner_and_transfer_identity(platform_version: &PlatformVersion) -> Identity {
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
        Identity::new_with_id_and_keys(
            Identifier::default(),
            [(17, owner_key), (18, transfer_key)].into(),
            platform_version,
        )
        .expect("identity")
    }

    #[tokio::test]
    async fn should_sign_an_owner_preferred_withdrawal_to_an_address_with_the_transfer_key() {
        let platform_version = PlatformVersion::latest();
        let identity = owner_and_transfer_identity(platform_version);

        let transition = IdentityCreditWithdrawalTransitionV1::try_from_identity(
            &identity,
            Some(CoreScript::from_bytes(vec![0x51])),
            1_000_000,
            Pooling::Never,
            1,
            0,
            SigningSigner,
            None,
            PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred,
            1,
            platform_version,
            None,
        )
        .await
        .expect("the TRANSFER key signs a withdrawal to a chosen address");

        let StateTransition::IdentityCreditWithdrawal(transition) = transition else {
            panic!("expected an identity credit withdrawal, got {transition:?}");
        };
        assert_eq!(transition.signature_public_key_id(), 18);
    }

    #[tokio::test]
    async fn should_sign_an_owner_preferred_withdrawal_without_an_address_with_the_owner_key() {
        let platform_version = PlatformVersion::latest();
        let identity = owner_and_transfer_identity(platform_version);

        let transition = IdentityCreditWithdrawalTransitionV1::try_from_identity(
            &identity,
            None,
            1_000_000,
            Pooling::Never,
            1,
            0,
            SigningSigner,
            None,
            PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred,
            1,
            platform_version,
            None,
        )
        .await
        .expect("the OWNER key signs a withdrawal to the payout address");

        let StateTransition::IdentityCreditWithdrawal(transition) = transition else {
            panic!("expected an identity credit withdrawal, got {transition:?}");
        };
        assert_eq!(transition.signature_public_key_id(), 17);
    }

    #[tokio::test]
    async fn should_refuse_an_owner_preferred_withdrawal_to_an_address_without_a_transfer_key() {
        let platform_version = PlatformVersion::latest();
        let owner_key = IdentityPublicKey::from(IdentityPublicKeyV0 {
            id: 17,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::CRITICAL,
            ..Default::default()
        });
        let identity = Identity::new_with_id_and_keys(
            Identifier::default(),
            [(17, owner_key)].into(),
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
        .expect_err("only an OWNER key is left, and it cannot sign this withdrawal");

        assert_owner_script_refusal(error, &output_script);
    }

    #[tokio::test]
    async fn should_refuse_an_owner_only_withdrawal_to_an_address_before_signing() {
        let platform_version = PlatformVersion::latest();
        let identity = owner_and_transfer_identity(platform_version);
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
            PreferredKeyPurposeForSigningWithdrawal::OwnerOnly,
            1,
            platform_version,
            None,
        )
        .await
        .expect_err("OWNER only cannot sign a withdrawal with an output script");

        assert_owner_script_refusal(error, &output_script);
    }

    /// A signer holding only the keys in `available`.
    #[derive(Debug)]
    struct SelectiveSigner {
        available: Vec<u32>,
    }

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for SelectiveSigner {
        async fn sign(
            &self,
            key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            if self.can_sign_with(key) {
                Ok(BinaryData::new(vec![0; 65]))
            } else {
                Err(ProtocolError::Generic(format!(
                    "key {} is not held",
                    key.id()
                )))
            }
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("identity credit withdrawals do not create an address witness");
        }

        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            self.available.contains(&key.id())
        }
    }

    /// OWNER keys 17 and 20, TRANSFER keys 18 and 19, all CRITICAL.
    fn identity_with_two_keys_per_purpose(platform_version: &PlatformVersion) -> Identity {
        let key = |id: u32, purpose: Purpose| {
            (
                id,
                IdentityPublicKey::from(IdentityPublicKeyV0 {
                    id,
                    purpose,
                    security_level: SecurityLevel::CRITICAL,
                    ..Default::default()
                }),
            )
        };
        Identity::new_with_id_and_keys(
            Identifier::default(),
            [
                key(17, Purpose::OWNER),
                key(18, Purpose::TRANSFER),
                key(19, Purpose::TRANSFER),
                key(20, Purpose::OWNER),
            ]
            .into(),
            platform_version,
        )
        .expect("identity")
    }

    #[tokio::test]
    async fn should_pick_the_first_key_the_signer_holds_in_every_automatic_selection() {
        let platform_version = PlatformVersion::latest();
        let identity = identity_with_two_keys_per_purpose(platform_version);
        let script = || Some(CoreScript::from_bytes(vec![0x51]));
        for (case, (preference, output_script, available, expected)) in [
            (
                PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred,
                None,
                vec![20, 19],
                20,
            ),
            (
                PreferredKeyPurposeForSigningWithdrawal::OwnerPreferred,
                script(),
                vec![17, 19],
                19,
            ),
            (
                PreferredKeyPurposeForSigningWithdrawal::TransferPreferred,
                None,
                vec![17, 19],
                19,
            ),
            (
                PreferredKeyPurposeForSigningWithdrawal::Any,
                None,
                vec![17, 19],
                19,
            ),
            (
                PreferredKeyPurposeForSigningWithdrawal::OwnerOnly,
                None,
                vec![18, 20],
                20,
            ),
            (
                PreferredKeyPurposeForSigningWithdrawal::TransferOnly,
                None,
                vec![17, 19],
                19,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let label = format!(
                "case {case}: script={} held={available:?}",
                output_script.is_some()
            );
            let transition = IdentityCreditWithdrawalTransitionV1::try_from_identity(
                &identity,
                output_script,
                1_000_000,
                Pooling::Never,
                1,
                0,
                SelectiveSigner { available },
                None,
                preference,
                1,
                platform_version,
                None,
            )
            .await
            .unwrap_or_else(|error| panic!("{label}: {error:?}"));

            let StateTransition::IdentityCreditWithdrawal(transition) = transition else {
                panic!("{label}: expected an identity credit withdrawal, got {transition:?}");
            };
            assert_eq!(transition.signature_public_key_id(), expected, "{label}");
        }
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
