#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod proved;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use std::convert::TryFrom;

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::Identity;
use crate::prelude::{Identifier, UserFeeIncrease};

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::state_transition::AssetLockProved;
use crate::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use crate::version::PlatformVersion;
use crate::ProtocolError;

#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase"),
    serde(try_from = "IdentityCreateTransitionV0Inner")
)]
// There is a problem deriving bincode for a borrowed vector
// Hence we set to do it somewhat manually inside the PlatformSignable proc macro
// Instead of inside of bincode_derive
#[platform_signable(derive_bincode_with_borrowed_vec)]
#[derive(Default)]
pub struct IdentityCreateTransitionV0 {
    // When signing, we don't sign the signatures for keys
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    pub asset_lock_proof: AssetLockProof,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(skip))]
    #[platform_signable(exclude_from_sig_hash)]
    pub identity_id: Identifier,
}

#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Deserialize),
    serde(rename_all = "camelCase")
)]
struct IdentityCreateTransitionV0Inner {
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreation>,
    asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    user_fee_increase: UserFeeIncrease,
    signature: BinaryData,
}

impl TryFrom<IdentityCreateTransitionV0Inner> for IdentityCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateTransitionV0Inner) -> Result<Self, Self::Error> {
        let IdentityCreateTransitionV0Inner {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            signature,
        } = value;
        let identity_id = asset_lock_proof.create_identifier()?;
        Ok(Self {
            public_keys,
            asset_lock_proof,
            user_fee_increase,
            signature,
            identity_id,
        })
    }
}

impl IdentityCreateTransitionV0 {
    pub fn try_from_identity_v0(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
    ) -> Result<Self, ProtocolError> {
        let mut identity_create_transition = IdentityCreateTransitionV0::default();

        let public_keys = identity
            .public_keys()
            .iter()
            .map(|(_, public_key)| public_key.into())
            .collect::<Vec<IdentityPublicKeyInCreation>>();
        identity_create_transition.set_public_keys(public_keys);

        identity_create_transition
            .set_asset_lock_proof(asset_lock_proof)
            .map_err(ProtocolError::from)?;

        Ok(identity_create_transition)
    }

    pub fn try_from_identity(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transition_conversion_versions
            .identity_to_identity_create_transition
        {
            0 => Self::try_from_identity_v0(identity, asset_lock_proof),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreateTransitionV0::try_from_identity".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
