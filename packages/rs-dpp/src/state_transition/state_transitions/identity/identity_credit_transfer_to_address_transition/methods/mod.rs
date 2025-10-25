mod v0;

#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::KeyOfType;
use crate::state_transition::identity_credit_transfer_to_address_transition::IdentityCreditTransferToAddressTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{signer::Signer, Identity, IdentityPublicKey},
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::{
        identity_credit_transfer_to_address_transition::v0::IdentityCreditTransferToAddressTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

impl IdentityCreditTransferToAddressTransitionMethodsV0
    for IdentityCreditTransferToAddressTransition
{
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity<S: Signer>(
        identity: &Identity,
        to_recipient_keys: BTreeMap<KeyOfType, Credits>,
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
            0 => Ok(
                IdentityCreditTransferToAddressTransitionV0::try_from_identity(
                    identity,
                    to_recipient_keys,
                    user_fee_increase,
                    signer,
                    signing_withdrawal_key_to_use,
                    nonce,
                    platform_version,
                    version,
                )?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditTransferToAddressTransition::try_from_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
