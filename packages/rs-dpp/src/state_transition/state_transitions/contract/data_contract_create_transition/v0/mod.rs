mod identity_signed;
mod state_transition_like;
mod types;
pub(crate) mod v0_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::{data_contract::DataContract, identity::KeyID, ProtocolError};

use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::serialized_version::TransitionContractFormat;
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::state_transitions::contract::stored_contract_format::decode_through;
#[cfg(feature = "serde-conversion")]
use crate::state_transition::state_transitions::contract::stored_contract_format::deserialize_transition_contract_format;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

use crate::state_transition::StateTransition;
use crate::version::PlatformVersion;

///DataContractCreateTransitionV0 has the same encoding structure

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
/// Its decoding refuses a contract in format version 2, the stored format, which only state
/// holds, at the format's tag, so none of its compiled types is built: bincode through a private
/// copy of the struct, serde through the contract field's deserializer; both read the contract as
/// a `TransitionContractFormat`, which says what each reads before the tag.
#[derive(Debug, Clone, Encode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractCreateTransitionV0 {
    #[cfg_attr(
        feature = "serde-conversion",
        serde(deserialize_with = "deserialize_transition_contract_format")
    )]
    pub data_contract: DataContractInSerializationFormat,
    pub identity_nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

/// [`DataContractCreateTransitionV0`] as decoding reads it: the same fields in the same order,
/// its contract read as a [`TransitionContractFormat`].
#[derive(Decode, DecodeUntrusted)]
struct DataContractCreateTransitionV0Decoded {
    data_contract: TransitionContractFormat,
    identity_nonce: IdentityNonce,
    user_fee_increase: UserFeeIncrease,
    signature_public_key_id: KeyID,
    signature: BinaryData,
}

impl From<DataContractCreateTransitionV0Decoded> for DataContractCreateTransitionV0 {
    fn from(decoded: DataContractCreateTransitionV0Decoded) -> Self {
        let DataContractCreateTransitionV0Decoded {
            data_contract,
            identity_nonce,
            user_fee_increase,
            signature_public_key_id,
            signature,
        } = decoded;
        Self {
            data_contract: data_contract.into(),
            identity_nonce,
            user_fee_increase,
            signature_public_key_id,
            signature,
        }
    }
}

decode_through!(
    DataContractCreateTransitionV0,
    DataContractCreateTransitionV0Decoded
);

impl From<DataContractCreateTransitionV0> for StateTransition {
    fn from(value: DataContractCreateTransitionV0) -> Self {
        let transition: DataContractCreateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractCreateTransitionV0> for StateTransition {
    fn from(value: &DataContractCreateTransitionV0) -> Self {
        let transition: DataContractCreateTransition = value.clone().into();
        transition.into()
    }
}

impl TryFromPlatformVersioned<DataContract> for DataContractCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractCreateTransitionV0 {
            data_contract: value.try_into_platform_versioned(platform_version)?,
            identity_nonce: Default::default(),
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}

impl TryFromPlatformVersioned<CreatedDataContract> for DataContractCreateTransitionV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: CreatedDataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let (data_contract, identity_nonce) = value.data_contract_and_identity_nonce();
        Ok(DataContractCreateTransitionV0 {
            data_contract: data_contract.try_into_platform_versioned(platform_version)?,
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}
