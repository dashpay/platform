use std::collections::BTreeMap;

use platform_value::{IntegerReplacementType, ReplacementType, Value};

use crate::{state_transition::StateTransitionFieldTypes, ProtocolError};

use crate::state_transition::masternode_vote_transition::fields::*;
use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::StateTransitionValueConvert;

use platform_version::version::PlatformVersion;

impl<'a> StateTransitionValueConvert<'a> for MasternodeVoteTransitionV0 {
    fn from_object(
        raw_object: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        platform_value::from_value(raw_object).map_err(ProtocolError::ValueError)
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
        let mut value = platform_value::to_value(self)?;
        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }
        Ok(value)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut value = platform_value::to_value(self)?;
        if skip_signature {
            value
                .remove_values_matching_paths(Self::signature_property_paths())
                .map_err(ProtocolError::ValueError)?;
        }
        Ok(value)
    }

    // Override to_canonical_cleaned_object to manage add_public_keys individually
    fn to_canonical_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        self.to_cleaned_object(skip_signature)
    }
}
