use crate::state_transition::shield_from_identity_transition::methods::ShieldFromIdentityTransitionMethodsV0;
use crate::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, signer::Signer, Identity,
        IdentityPublicKey, KeyType, Purpose, SecurityLevel,
    },
    prelude::{IdentityNonce, UserFeeIncrease},
    shielded::SerializedAction,
    state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition},
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

impl ShieldFromIdentityTransitionMethodsV0 for ShieldFromIdentityTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_bundle_with_identity_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        amount: u64,
        actions: Vec<SerializedAction>,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        signing_transfer_key_to_use: Option<&IdentityPublicKey>,
        nonce: IdentityNonce,
        _platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError> {
        let mut transition: StateTransition = ShieldFromIdentityTransitionV0 {
            identity_id: identity.id(),
            amount,
            actions,
            anchor,
            proof,
            binding_signature,
            nonce,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        let identity_public_key = match signing_transfer_key_to_use {
            Some(key) => {
                if signer.can_sign_with(key) {
                    key
                } else {
                    return Err(
                        ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                            "specified transfer public key cannot be used for signing".to_string(),
                        ),
                    );
                }
            }
            None => identity
                .get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                )
                .ok_or_else(|| {
                    ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                        "no transfer public key".to_string(),
                    )
                })?,
        };

        tracing::debug!(
            key_id = identity_public_key.id(),
            "shield from identity: signing with transfer key"
        );

        transition
            .sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )
            .await?;

        Ok(transition)
    }
}
