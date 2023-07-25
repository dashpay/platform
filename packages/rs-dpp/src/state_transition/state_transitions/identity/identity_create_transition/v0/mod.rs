#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use std::convert::{TryFrom, TryInto};
use std::process::id;

use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreationSignable;
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{BinaryData, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::identity::signer::Signer;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::identity::Identity;
use crate::prelude::Identifier;

use crate::identity::accessors::IdentityGettersV0;
use crate::state_transition::identity_create_transition::v0::v0_methods::IdentityCreateTransitionV0Methods;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::{
    StateTransition, StateTransitionFieldTypes, StateTransitionLike, StateTransitionType,
};
use crate::version::{FeatureVersion, PlatformVersion};
use crate::{BlsModule, NonConsensusError, ProtocolError};

#[derive(Debug, Clone, PartialEq, PlatformDeserialize, PlatformSerialize, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase"),
    serde(try_from = "IdentityCreateTransitionV0Inner")
)]
#[platform_serialize(allow_nested)]
#[platform_error_type(ProtocolError)]
pub struct IdentityCreateTransitionV0 {
    // The signable
    #[platform_signable(into = "Vec<IdentityPublicKeyInCreationSignable>")]
    pub public_keys: Vec<IdentityPublicKeyInCreation>,
    pub asset_lock_proof: AssetLockProof,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(skip))]
    #[platform_signable(exclude_from_sig_hash)]
    pub identity_id: Identifier,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateTransitionV0Inner {
    // Own ST fields
    public_keys: Vec<IdentityPublicKeyInCreation>,
    asset_lock_proof: AssetLockProof,
    // Generic identity ST fields
    protocol_version: u32,
    signature: BinaryData,
}

impl TryFrom<IdentityCreateTransitionV0Inner> for IdentityCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from(value: IdentityCreateTransitionV0Inner) -> Result<Self, Self::Error> {
        let IdentityCreateTransitionV0Inner {
            public_keys,
            asset_lock_proof,
            protocol_version,
            signature,
        } = value;
        let identity_id = asset_lock_proof.create_identifier()?;
        Ok(Self {
            public_keys,
            asset_lock_proof,
            signature,
            identity_id,
        })
    }
}

//todo: there shouldn't be a default
impl Default for IdentityCreateTransitionV0 {
    fn default() -> Self {
        Self {
            public_keys: Default::default(),
            asset_lock_proof: Default::default(),
            identity_id: Default::default(),
            signature: Default::default(),
        }
    }
}

impl IdentityCreateTransitionV0 {
    fn try_from_identity_v0(
        identity: Identity,
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
        identity: Identity,
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
