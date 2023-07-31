use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueRemoveFromMapHelper, BTreeValueRemoveInnerValueFromMapHelper,
};
use platform_value::{BinaryData, Bytes32, IntegerReplacementType, ReplacementType, Value};
use serde::{Deserialize, Serialize};

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    Convertible, NonConsensusError, ProtocolError,
};

use crate::prelude::AssetLockProof;
use crate::serialization::{PlatformDeserializable, Signable};
use crate::state_transition::identity_topup_transition::fields::*;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::StateTransitionValueConvert;
use bincode::{config, Decode, Encode};
use platform_version::version::PlatformVersion;

impl StateTransitionValueConvert for IdentityTopUpTransitionV0 {
    fn from_object(
        mut raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let signature = raw_object
            .get_optional_binary_data(SIGNATURE)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default();
        let identity_id = Identifier::from(
            raw_object
                .get_hash256(IDENTITY_ID)
                .map_err(ProtocolError::ValueError)?,
        );

        let raw_asset_lock_proof = raw_object
            .get_value(ASSET_LOCK_PROOF)
            .map_err(ProtocolError::ValueError)?;
        let asset_lock_proof = AssetLockProof::try_from(raw_asset_lock_proof)?;

        Ok(IdentityTopUpTransitionV0 {
            signature,
            identity_id,
            asset_lock_proof,
        })
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }

    fn from_value_map(
        mut raw_value_map: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        todo()
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        Ok(value)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value: Value = platform_value::to_value(self)?;

        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }

        Ok(value)
    }
}
