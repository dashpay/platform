/// transformer
pub mod transformer;

use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, IdentityV0, PartialIdentity};

use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use serde::{Deserialize, Serialize};
use dpp::prelude::FeeMultiplier;

/// action v0
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreateTransitionActionV0 {
    /// public keys
    pub public_keys: Vec<IdentityPublicKey>,
    /// initial balance amount
    pub initial_balance_amount: u64,
    /// identity id
    pub identity_id: Identifier,
    /// asset lock outpoint
    pub asset_lock_outpoint: Bytes36,
    /// fee multiplier
    pub fee_multiplier: FeeMultiplier,
}

impl From<IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(initial_balance_amount),
            revision: None,

            not_found_public_keys: Default::default(),
        }
    }
}

impl From<&IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: &IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: *identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(*initial_balance_amount),
            revision: None,

            not_found_public_keys: Default::default(),
        }
    }
}

/// action v0
pub trait IdentityFromIdentityCreateTransitionActionV0 {
    /// try from
    fn try_from_identity_create_transition_action_v0(
        value: IdentityCreateTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// try from borrowed
    fn try_from_borrowed_identity_create_transition_action_v0(
        value: &IdentityCreateTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl IdentityFromIdentityCreateTransitionActionV0 for Identity {
    fn try_from_identity_create_transition_action_v0(
        value: IdentityCreateTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            public_keys,
            ..
        } = value;
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityV0 {
                id: identity_id,
                public_keys: public_keys.into_iter().map(|key| (key.id(), key)).collect(),
                balance: initial_balance_amount,
                revision: 0,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::try_from_identity_create_transition_action_v0".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
    fn try_from_borrowed_identity_create_transition_action_v0(
        value: &IdentityCreateTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreateTransitionActionV0 {
            initial_balance_amount,
            identity_id,
            public_keys,
            ..
        } = value;
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(IdentityV0 {
                id: *identity_id,
                public_keys: public_keys
                    .iter()
                    .map(|key| (key.id(), key.clone()))
                    .collect(),
                balance: *initial_balance_amount,
                revision: 0,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::try_from_borrowed_identity_create_transition_action_v0"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
