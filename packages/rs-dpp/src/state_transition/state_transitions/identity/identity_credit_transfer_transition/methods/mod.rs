mod v0;
pub use v0::*;

use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{signer::Signer, Identity, IdentityPublicKey},
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::{
        identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

impl IdentityCreditTransferTransitionMethodsV0 for IdentityCreditTransferTransition {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        to_identity_with_identifier: Identifier,
        amount: u64,
        user_fee_increase: UserFeeIncrease,
        signer: S,
        signing_withdrawal_key_to_use: Option<&IdentityPublicKey>,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        match version.unwrap_or(
            platform_version
                .dpp
                .state_transition_conversion_versions
                .identity_to_identity_transfer_transition,
        ) {
            0 => Ok(IdentityCreditTransferTransitionV0::try_from_identity(
                identity,
                to_identity_with_identifier,
                amount,
                user_fee_increase,
                signer,
                signing_withdrawal_key_to_use,
                nonce,
                platform_version,
                version,
            )?),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditTransferTransition::try_from_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
