#[cfg(feature = "state-transition-signing")]
use crate::{
    consensus::basic::identity::{
        IdentityCreditTransferToSelfError, InvalidIdentityCreditTransferAmountError,
    },
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
        platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let min_transfer_amount = platform_version
            .fee_version
            .state_transition_min_fees
            .credit_transfer;

        if identity.id() == to_identity_with_identifier {
            let error = IdentityCreditTransferToSelfError::default();
            return Err(ProtocolError::ConsensusError(Box::new(error.into())));
        }

        if amount < min_transfer_amount {
            let error = InvalidIdentityCreditTransferAmountError::new(amount, min_transfer_amount);
            return Err(ProtocolError::ConsensusError(Box::new(error.into())));
        }

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

#[cfg(all(test, feature = "state-transition-signing"))]
mod tests {
    use crate::address_funds::AddressWitness;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::signer::Signer;
    use crate::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use crate::state_transition::identity_credit_transfer_transition::methods::IdentityCreditTransferTransitionMethodsV0;
    use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use crate::ProtocolError;
    use platform_value::BinaryData;
    use platform_value::Identifier;
    use platform_version::version::LATEST_PLATFORM_VERSION;

    #[derive(Debug)]
    struct TestIdentitySigner {
        allowed_key_id: u32,
    }

    impl Signer<IdentityPublicKey> for TestIdentitySigner {
        fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Ok(vec![0; 65].into())
        }

        fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::NotSupported(
                "sign_create_witness is not used in these tests".to_string(),
            ))
        }

        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            key.id() == self.allowed_key_id
        }
    }

    fn transfer_key(key_id: u32) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::TRANSFER,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: vec![2; 33].into(),
            disabled_at: None,
        }
        .into()
    }

    fn identity_with_transfer_key(identity_id: Identifier, key_id: u32) -> Identity {
        let mut identity =
            Identity::default_versioned(LATEST_PLATFORM_VERSION).expect("expected identity");
        identity.set_id(identity_id);
        identity.add_public_key(transfer_key(key_id));
        identity
    }

    #[test]
    fn should_return_identity_credit_transfer_to_self_error_when_recipient_is_sender() {
        let identity_id = Identifier::random();
        let identity = identity_with_transfer_key(identity_id, 1);
        let min_transfer_amount = LATEST_PLATFORM_VERSION
            .fee_version
            .state_transition_min_fees
            .credit_transfer;

        let result = IdentityCreditTransferTransitionV0::try_from_identity(
            &identity,
            identity_id,
            min_transfer_amount,
            0,
            TestIdentitySigner { allowed_key_id: 1 },
            None,
            1,
            LATEST_PLATFORM_VERSION,
            None,
        );

        match result {
            Err(ProtocolError::ConsensusError(consensus_error)) => {
                assert!(matches!(
                    consensus_error.as_ref(),
                    ConsensusError::BasicError(BasicError::IdentityCreditTransferToSelfError(_))
                ));
            }
            other => panic!("expected to-self consensus error, got {other:?}"),
        }
    }

    #[test]
    fn should_return_invalid_identity_credit_transfer_amount_error_when_below_minimum() {
        let identity_id = Identifier::random();
        let identity = identity_with_transfer_key(identity_id, 1);
        let min_transfer_amount = LATEST_PLATFORM_VERSION
            .fee_version
            .state_transition_min_fees
            .credit_transfer;
        assert!(
            min_transfer_amount > 0,
            "minimum transfer amount must be positive"
        );
        let below_min_amount = min_transfer_amount - 1;

        let result = IdentityCreditTransferTransitionV0::try_from_identity(
            &identity,
            Identifier::random(),
            below_min_amount,
            0,
            TestIdentitySigner { allowed_key_id: 1 },
            None,
            1,
            LATEST_PLATFORM_VERSION,
            None,
        );

        match result {
            Err(ProtocolError::ConsensusError(consensus_error)) => {
                assert!(matches!(
                    consensus_error.as_ref(),
                    ConsensusError::BasicError(
                        BasicError::InvalidIdentityCreditTransferAmountError(error)
                    ) if error.amount() == below_min_amount
                        && error.min_amount() == min_transfer_amount
                ));
            }
            other => panic!("expected invalid amount consensus error, got {other:?}"),
        }
    }

    #[test]
    fn should_succeed_when_transfer_amount_equals_minimum() {
        let identity_id = Identifier::random();
        let identity = identity_with_transfer_key(identity_id, 1);
        let min_transfer_amount = LATEST_PLATFORM_VERSION
            .fee_version
            .state_transition_min_fees
            .credit_transfer;

        let result = IdentityCreditTransferTransitionV0::try_from_identity(
            &identity,
            Identifier::random(),
            min_transfer_amount,
            0,
            TestIdentitySigner { allowed_key_id: 1 },
            None,
            1,
            LATEST_PLATFORM_VERSION,
            None,
        );

        assert!(result.is_ok(), "expected boundary amount to be valid");
    }
}
