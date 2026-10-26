mod identity_signed;
mod state_transition_like;
mod types;
pub(crate) mod v1_methods;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use platform_serialization_derive::PlatformSignable;

use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::contract_group::{ContractGroupMembership, ContractGroupRegistration};
use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::serialized_version::TransitionContractFormat;
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::state_transitions::contract::stored_contract_format::decode_through;
use crate::state_transition::StateTransition;
use crate::version::PlatformVersion;
use crate::{data_contract::DataContract, identity::KeyID, ProtocolError};
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

/// Version 1 of the data contract create transition: version 0 plus contract groups.
///
/// `contract_group` registers a new contract group owned by the transition's owner (or by the
/// owners it names); the group id is derived from the owner id and the identity nonce.
/// `contract_group_memberships` adds the created contract, one of its document types, or one of
/// its tokens to contract groups the owner may add to: groups registered earlier by an owner, or
/// the group registered by this transition. Both are empty on a plain contract creation.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
/// Decoded through [`DataContractCreateTransitionV1Decoded`], which refuses the stored contract
/// format at its tag.
#[derive(Debug, Clone, Encode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractCreateTransitionV1 {
    pub data_contract: DataContractInSerializationFormat,
    pub identity_nonce: IdentityNonce,
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub contract_group: Option<ContractGroupRegistration>,
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub contract_group_memberships: Vec<ContractGroupMembership>,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

/// [`DataContractCreateTransitionV1`] as decoding reads it: the same fields in the same order,
/// its contract read as a [`TransitionContractFormat`].
#[derive(Decode, DecodeUntrusted)]
struct DataContractCreateTransitionV1Decoded {
    data_contract: TransitionContractFormat,
    identity_nonce: IdentityNonce,
    contract_group: Option<ContractGroupRegistration>,
    contract_group_memberships: Vec<ContractGroupMembership>,
    user_fee_increase: UserFeeIncrease,
    signature_public_key_id: KeyID,
    signature: BinaryData,
}

impl From<DataContractCreateTransitionV1Decoded> for DataContractCreateTransitionV1 {
    fn from(decoded: DataContractCreateTransitionV1Decoded) -> Self {
        let DataContractCreateTransitionV1Decoded {
            data_contract,
            identity_nonce,
            contract_group,
            contract_group_memberships,
            user_fee_increase,
            signature_public_key_id,
            signature,
        } = decoded;
        Self {
            data_contract: data_contract.into(),
            identity_nonce,
            contract_group,
            contract_group_memberships,
            user_fee_increase,
            signature_public_key_id,
            signature,
        }
    }
}

decode_through!(
    DataContractCreateTransitionV1,
    DataContractCreateTransitionV1Decoded
);

impl From<DataContractCreateTransitionV1> for StateTransition {
    fn from(value: DataContractCreateTransitionV1) -> Self {
        let transition: DataContractCreateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractCreateTransitionV1> for StateTransition {
    fn from(value: &DataContractCreateTransitionV1) -> Self {
        let transition: DataContractCreateTransition = value.clone().into();
        transition.into()
    }
}

impl TryFromPlatformVersioned<DataContract> for DataContractCreateTransitionV1 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractCreateTransitionV1 {
            data_contract: value.try_into_platform_versioned(platform_version)?,
            identity_nonce: Default::default(),
            contract_group: None,
            contract_group_memberships: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}

impl TryFromPlatformVersioned<CreatedDataContract> for DataContractCreateTransitionV1 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: CreatedDataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let (data_contract, identity_nonce) = value.data_contract_and_identity_nonce();
        Ok(DataContractCreateTransitionV1 {
            data_contract: data_contract.try_into_platform_versioned(platform_version)?,
            identity_nonce,
            contract_group: None,
            contract_group_memberships: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}
