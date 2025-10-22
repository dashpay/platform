mod v0;

use std::collections::BTreeMap;
pub use v0::*;

use crate::state_transition::identity_credit_transfer_to_single_key_transition::IdentityCreditTransferToSingleKeyTransition;
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{signer::Signer, Identity, IdentityPublicKey},
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::{
        identity_credit_transfer_to_single_key_transition::v0::IdentityCreditTransferToSingleKeyTransitionV0,
        StateTransition,
    },
    ProtocolError,
};
#[cfg(feature = "state-transition-signing")]
use platform_value::Identifier;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};
use crate::fee::Credits;
use crate::identity::KeyOfType;

impl IdentityCreditTransferToSingleKeyTransitionMethodsV0 for IdentityCreditTransferToSingleKeyTransition {
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
            0 => Ok(IdentityCreditTransferToSingleKeyTransitionV0::try_from_identity(
                identity,
                to_recipient_keys,
                user_fee_increase,
                signer,
                signing_withdrawal_key_to_use,
                nonce,
                platform_version,
                version,
            )?),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditTransferToSingleKeyTransition::try_from_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
