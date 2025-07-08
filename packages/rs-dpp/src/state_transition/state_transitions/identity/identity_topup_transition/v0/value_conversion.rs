use std::collections::BTreeMap;
use std::convert::TryFrom;

use platform_value::{IntegerReplacementType, ReplacementType, Value};

use crate::{prelude::Identifier, state_transition::StateTransitionFieldTypes, ProtocolError};

use crate::prelude::AssetLockProof;

use crate::state_transition::identity_topup_transition::fields::*;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::StateTransitionValueConvert;

use crate::state_transition::state_transitions::common_fields::property_names::USER_FEE_INCREASE;
use platform_version::version::PlatformVersion;

impl StateTransitionValueConvert<'_> for IdentityTopUpTransitionV0 {
    fn from_object(
        raw_object: Value,
        _platform_version: &PlatformVersion,
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

        let user_fee_increase = raw_object
            .get_optional_integer(USER_FEE_INCREASE)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default();

        Ok(IdentityTopUpTransitionV0 {
            signature,
            identity_id,
            asset_lock_proof,
            user_fee_increase,
        })
    }

    fn clean_value(value: &mut Value) -> Result<(), ProtocolError> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_at_paths(BINARY_FIELDS, ReplacementType::BinaryBytes)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }

    fn from_value_map(
        raw_value_map: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let value: Value = raw_value_map.into();
        Self::from_object(value, platform_version)
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
