/// transformer
pub mod transformer;

use dpp::identifier::Identifier;
use dpp::identity::{IdentityPublicKey, IdentityV0, KeyOfType, PartialIdentity};
use std::collections::BTreeMap;
use dpp::balances::credits::RemainingCredits;
use dpp::fee::Credits;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::prelude::{KeyOfTypeNonce, UserFeeIncrease};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// action v0
#[derive(Debug, Clone)]
pub struct IdentityCreateFromAddressesTransitionActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<KeyOfType, (KeyOfTypeNonce, RemainingCredits)>,
    /// public keys
    pub public_keys: Vec<IdentityPublicKey>,
    /// identity id
    pub identity_id: Identifier,
    /// the amount that the identity will receive
    pub fund_identity_amount: Credits,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

impl From<IdentityCreateFromAddressesTransitionActionV0> for PartialIdentity {
    fn from(value: IdentityCreateFromAddressesTransitionActionV0) -> Self {
        let IdentityCreateFromAddressesTransitionActionV0 {
            fund_identity_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(fund_identity_amount),
            revision: None,

            not_found_public_keys: Default::default(),
        }
    }
}

impl From<&IdentityCreateFromAddressesTransitionActionV0> for PartialIdentity {
    fn from(value: &IdentityCreateFromAddressesTransitionActionV0) -> Self {
        let IdentityCreateFromAddressesTransitionActionV0 {
            fund_identity_amount,
            identity_id,
            ..
        } = value;
        PartialIdentity {
            id: *identity_id,
            loaded_public_keys: Default::default(), //no need to load public keys
            balance: Some(
                *fund_identity_amount
            ),
            revision: None,

            not_found_public_keys: Default::default(),
        }
    }
}

/// action v0
pub trait IdentityFromIdentityCreateFromAddressesTransitionActionV0 {
    /// try from borrowed
    fn try_from_borrowed_identity_create_from_addresses_transition_action_v0(
        value: &IdentityCreateFromAddressesTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl IdentityFromIdentityCreateFromAddressesTransitionActionV0 for Identity {
    fn try_from_borrowed_identity_create_from_addresses_transition_action_v0(
        value: &IdentityCreateFromAddressesTransitionActionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let IdentityCreateFromAddressesTransitionActionV0 {
            fund_identity_amount,
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
                balance: *fund_identity_amount,
                revision: 0,
            }
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::try_from_borrowed_identity_create_from_addresses_transition_action_v0"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
