mod v0;

use crate::consensus::basic::identity::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError;
use crate::identity::core_script::CoreScript;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::{IdentityPublicKey, Purpose};

use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::FeatureVersion;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;

#[cfg(feature = "state-transition-signing")]
use crate::prelude::{IdentityNonce, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
#[cfg(feature = "state-transition-signing")]
use crate::version::PlatformVersion;
#[cfg(feature = "state-transition-signing")]
use crate::withdrawal::Pooling;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;

/// Refuse an explicit withdrawal output script for an OWNER signing key.
pub fn validate_withdrawal_output_script(
    signing_key: &IdentityPublicKey,
    output_script: Option<&CoreScript>,
) -> Result<(), WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError> {
    if let Some(output_script) = output_script.filter(|_| signing_key.purpose() == Purpose::OWNER) {
        return Err(
            WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError::new(
                output_script.clone(),
                signing_key.id(),
            ),
        );
    }

    Ok(())
}

impl IdentityCreditWithdrawalTransitionMethodsV0 for IdentityCreditWithdrawalTransition {
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
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match version.unwrap_or(
            platform_version
                .dpp
                .state_transition_conversion_versions
                .identity_to_identity_withdrawal_transition,
        ) {
            1 => Ok(IdentityCreditWithdrawalTransitionV1::try_from_identity(
                identity,
                output_script,
                amount,
                pooling,
                core_fee_per_byte,
                user_fee_increase,
                signer,
                signing_withdrawal_key_to_use,
                preferred_key_purpose_for_signing_withdrawal,
                nonce,
                platform_version,
                version,
            )
            .await?),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditWithdrawalTransition::try_from_identity".to_string(),
                known_versions: vec![1],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;

    fn signing_key(purpose: Purpose) -> IdentityPublicKey {
        IdentityPublicKeyV0 {
            id: 17,
            purpose,
            ..Default::default()
        }
        .into()
    }

    #[test]
    fn should_refuse_an_owner_key_with_an_output_script() {
        let output_script = CoreScript::from_bytes(vec![0x51]);
        let error =
            validate_withdrawal_output_script(&signing_key(Purpose::OWNER), Some(&output_script))
                .expect_err("OWNER cannot sign a withdrawal with an output script");

        assert_eq!(error.output_script(), &output_script);
        assert_eq!(error.key_id(), 17);
    }

    #[test]
    fn should_allow_an_owner_key_without_an_output_script() {
        assert_eq!(
            validate_withdrawal_output_script(&signing_key(Purpose::OWNER), None),
            Ok(())
        );
    }

    #[test]
    fn should_allow_an_output_script_with_non_owner_keys() {
        let output_script = CoreScript::from_bytes(vec![0x51]);
        for purpose in [Purpose::TRANSFER, Purpose::AUTHENTICATION] {
            assert_eq!(
                validate_withdrawal_output_script(&signing_key(purpose), Some(&output_script)),
                Ok(())
            );
        }
    }
}
