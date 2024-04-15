/// transformer
pub mod transformer;

use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, IdentityV0, PartialIdentity};

use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::platform_value::Bytes36;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// action v0
#[derive(Debug, Clone)]
pub struct IdentityCreateTransitionActionV0 {
    /// The state transition signable bytes hash
    pub signable_bytes_hasher: SignableBytesHasher,
    /// public keys
    pub public_keys: Vec<IdentityPublicKey>,
    /// the initial balance amount is equal to the remaining asset lock value
    pub asset_lock_value_to_be_consumed: AssetLockValue,
    /// identity id
    pub identity_id: Identifier,
    /// asset lock outpoint
    pub asset_lock_outpoint: Bytes36,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

impl From<IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            asset_lock_value_to_be_consumed,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(asset_lock_value_to_be_consumed.remaining_credit_value()),
            revision: None,

            not_found_public_keys: Default::default(),
        }
    }
}

impl From<&IdentityCreateTransitionActionV0> for PartialIdentity {
    fn from(value: &IdentityCreateTransitionActionV0) -> Self {
        let IdentityCreateTransitionActionV0 {
            asset_lock_value_to_be_consumed,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: *identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(asset_lock_value_to_be_consumed.remaining_credit_value()),
            revision: None,

            not_found_public_keys: Default::default(),
        }
    }
}

/// action v0
pub trait IdentityFromIdentityCreateTransitionActionV0 {
    /// try from
    fn try_from_identity_create_transition_action_returning_asset_lock_value_v0(
        value: IdentityCreateTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, AssetLockValue), ProtocolError>
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
    fn try_from_identity_create_transition_action_returning_asset_lock_value_v0(
        value: IdentityCreateTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, AssetLockValue), ProtocolError> {
        let IdentityCreateTransitionActionV0 {
            asset_lock_value_to_be_consumed,
            identity_id,
            public_keys,
            ..
        } = value;
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok((
                IdentityV0 {
                    id: identity_id,
                    public_keys: public_keys.into_iter().map(|key| (key.id(), key)).collect(),
                    balance: asset_lock_value_to_be_consumed.remaining_credit_value(),
                    revision: 0,
                }
                .into(),
                asset_lock_value_to_be_consumed,
            )),
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
            asset_lock_value_to_be_consumed,
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
                balance: asset_lock_value_to_be_consumed.remaining_credit_value(),
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
