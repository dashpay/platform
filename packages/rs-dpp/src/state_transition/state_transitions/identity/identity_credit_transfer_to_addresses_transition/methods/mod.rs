mod v0;

#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;
pub use v0::*;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{signer::Signer, Identity, IdentityPublicKey},
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::{
        identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

impl IdentityCreditTransferToAddressesTransitionMethodsV0
    for IdentityCreditTransferToAddressesTransition
{
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        to_recipient_addresses: BTreeMap<PlatformAddress, Credits>,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
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
                IdentityCreditTransferToAddressesTransitionV0::try_from_identity(
                    identity,
                    to_recipient_addresses,
                    user_fee_increase,
                    signer,
                    signing_withdrawal_key_to_use,
                    nonce,
                    platform_version,
                    version,
                )
                .await?,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditTransferToAddressesTransition::try_from_identity"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
