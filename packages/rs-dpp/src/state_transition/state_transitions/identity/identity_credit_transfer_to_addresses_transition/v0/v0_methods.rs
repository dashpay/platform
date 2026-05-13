#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{
    address_funds_constructor_dispatch_error, consensus_errors_as_protocol_error,
    StateTransitionStructureValidation,
};
#[cfg(feature = "state-transition-signing")]
use crate::{
    identity::{
        accessors::IdentityGettersV0,
        identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, signer::Signer, Identity,
        IdentityPublicKey, KeyType, Purpose, SecurityLevel,
    },
    prelude::{IdentityNonce, UserFeeIncrease},
    state_transition::StateTransition,
    ProtocolError,
};

use crate::state_transition::identity_credit_transfer_to_addresses_transition::methods::IdentityCreditTransferToAddressesTransitionMethodsV0;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::GetDataContractSecurityLevelRequirementFn;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

impl IdentityCreditTransferToAddressesTransitionMethodsV0
    for IdentityCreditTransferToAddressesTransitionV0
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
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        tracing::debug!("try_from_identity: Started");
        tracing::debug!(identity_id = %identity.id(), "try_from_identity");
        tracing::debug!(recipient_addresses = ?to_recipient_addresses, has_signing_key = signing_withdrawal_key_to_use.is_some(), "try_from_identity inputs");

        let transition_v0 = IdentityCreditTransferToAddressesTransitionV0 {
            identity_id: identity.id(),
            recipient_addresses: to_recipient_addresses,
            nonce,
            user_fee_increase,
            signature_public_key_id: 0,
            signature: Default::default(),
        };

        if let Some(error) = address_funds_constructor_dispatch_error(
            StateTransitionType::IdentityCreditTransferToAddresses,
            platform_version,
        ) {
            return Err(error);
        }

        // Validate structure before .into() conversion and signing, since this transition
        // uses sign_external on the StateTransition rather than setting witnesses on the V0 struct.
        //
        // LOCKSTEP: this call resolves to the v0 structure check. If a future
        // v1 basic-structure is introduced for this transition, both the
        // drive-abci server dispatcher AND this SDK constructor must be
        // updated together (e.g. by routing through a versioned
        // `validate_basic_structure` wrapper as IdentityUpdate does).
        let validation_result = transition_v0.validate_structure(platform_version);
        if let Some(error) = consensus_errors_as_protocol_error(validation_result) {
            return Err(error);
        }

        let mut transition: StateTransition = transition_v0.into();

        let identity_public_key = match signing_withdrawal_key_to_use {
            Some(key) => {
                if signer.can_sign_with(key) {
                    key
                } else {
                    tracing::error!(
                        key_id = key.id(),
                        "try_from_identity: specified transfer key cannot be used for signing"
                    );
                    return Err(
                        ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                            "specified transfer public key cannot be used for signing".to_string(),
                        ),
                    );
                }
            }
            None => {
                tracing::debug!("try_from_identity: No signing key specified, searching for TRANSFER key (full_range, all_key_types, allow_disabled=true)");

                let key_result = identity.get_first_public_key_matching(
                    Purpose::TRANSFER,
                    SecurityLevel::full_range().into(),
                    KeyType::all_key_types().into(),
                    true,
                );

                tracing::debug!(
                    found = key_result.is_some(),
                    "try_from_identity: get_first_public_key_matching result"
                );

                key_result.ok_or_else(|| {
                    tracing::error!(total_keys = identity.public_keys().len(), "try_from_identity: No transfer public key found in identity");
                    for (key_id, key) in identity.public_keys() {
                        tracing::debug!(key_id, key_purpose = ?key.purpose(), "try_from_identity: identity key");
                    }
                    ProtocolError::DesiredKeyWithTypePurposeSecurityLevelMissing(
                        "no transfer public key".to_string(),
                    )
                })?
            }
        };

        tracing::debug!(
            key_id = identity_public_key.id(),
            "try_from_identity: Found identity public key"
        );
        tracing::debug!("try_from_identity: Calling transition.sign_external");

        match transition
            .sign_external(
                identity_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )
            .await
        {
            Ok(_) => tracing::debug!("try_from_identity: sign_external succeeded"),
            Err(e) => {
                tracing::error!(error = ?e, "try_from_identity: sign_external failed");
                return Err(e);
            }
        }

        tracing::debug!("try_from_identity: Successfully created and signed transition");
        Ok(transition)
    }
}
