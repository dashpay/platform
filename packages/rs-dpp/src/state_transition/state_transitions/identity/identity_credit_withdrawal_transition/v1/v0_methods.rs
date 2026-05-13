#[cfg(feature = "state-transition-signing")]
use crate::state_transition::consensus_errors_as_protocol_error;
#[cfg(feature = "state-transition-signing")]
use crate::{
    consensus::basic::state_transition::StateTransitionNotActiveError,
    consensus::ConsensusError,
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
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransitionType;

#[cfg(feature = "state-transition-signing")]
fn validate_basic_structure_dispatch(
    transition: &IdentityCreditWithdrawalTransitionV1,
    platform_version: &PlatformVersion,
) -> Result<crate::validation::SimpleConsensusValidationResult, ProtocolError> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .identity_credit_withdrawal_state_transition
        .basic_structure
    {
        Some(1) => Ok(transition.basic_structure_rules_v1(platform_version)),
        None => {
            let first_active_version = platform_version::version::PLATFORM_VERSIONS
                .iter()
                .find(|version| {
                    version
                        .drive_abci
                        .validation_and_processing
                        .state_transitions
                        .identity_credit_withdrawal_state_transition
                        .basic_structure
                        == Some(1)
                })
                .map(|version| version.protocol_version)
                .unwrap_or(platform_version.protocol_version);

            Err(ProtocolError::from(ConsensusError::from(
                StateTransitionNotActiveError::new(
                    StateTransitionType::IdentityCreditWithdrawal.to_string(),
                    platform_version.protocol_version,
                    first_active_version,
                ),
            )))
        }
        Some(0) => Err(ProtocolError::UnknownVersionMismatch {
            method: "IdentityCreditWithdrawalTransitionV1::validate_basic_structure".to_string(),
            known_versions: vec![1],
            received: 0,
        }),
        Some(version) => Err(ProtocolError::UnknownVersionMismatch {
            method: "IdentityCreditWithdrawalTransitionV1::validate_basic_structure".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}

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
        platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let transition_v1 = IdentityCreditWithdrawalTransitionV1 {
            identity_id: identity.id(),
            amount,
            core_fee_per_byte,
            pooling,
            output_script,
            nonce,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        };

        // Pre-signing structure check that delegates to the shared DPP-owned
        // v1 basic-structure validation. The drive-abci server dispatcher and
        // the enum wrapper route through the same rule function, so client and
        // server cannot drift.
        //
        // LOCKSTEP: hard-coded to the v1 basic-structure check. If a future v2
        // basic-structure rule is introduced for this transition, the DPP
        // method, the drive-abci dispatcher, AND this SDK constructor must be
        // updated together.
        let pre_validation_result =
            validate_basic_structure_dispatch(&transition_v1, platform_version)?;
        if let Some(error) = consensus_errors_as_protocol_error(pre_validation_result) {
            return Err(error);
        }

        let mut transition: StateTransition = transition_v1.into();

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
