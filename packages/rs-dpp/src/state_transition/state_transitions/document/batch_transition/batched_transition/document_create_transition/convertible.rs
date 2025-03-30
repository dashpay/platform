#[cfg(feature = "state-transition-json-conversion")]
use crate::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(feature = "state-transition-json-conversion")]
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use crate::prelude::DataContract;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use crate::state_transition::batch_transition::document_base_transition::v0::DocumentTransitionObjectLike;
#[cfg(feature = "state-transition-json-conversion")]
use crate::state_transition::batch_transition::document_create_transition::v0::BINARY_FIELDS;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use crate::state_transition::batch_transition::document_create_transition::DocumentCreateTransition;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
#[cfg(feature = "state-transition-value-conversion")]
use crate::state_transition::batch_transition::fields::property_names::STATE_TRANSITION_PROTOCOL_VERSION;
#[cfg(feature = "state-transition-json-conversion")]
use crate::state_transition::data_contract_update_transition::IDENTIFIER_FIELDS;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use crate::ProtocolError;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
#[cfg(feature = "state-transition-json-conversion")]
use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapReplacementPathHelper,
};
#[cfg(feature = "state-transition-json-conversion")]
use platform_value::ReplacementType;
#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
use platform_value::Value;
#[cfg(feature = "state-transition-json-conversion")]
use serde_json::Value as JsonValue;
#[cfg(feature = "state-transition-value-conversion")]
use std::collections::BTreeMap;

#[cfg(any(
    feature = "state-transition-json-conversion",
    feature = "state-transition-value-conversion"
))]
impl DocumentTransitionObjectLike for DocumentCreateTransition {
    #[cfg(feature = "state-transition-json-conversion")]
    fn from_json_object(
        json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let value: Value = json_value.into();
        let mut map = value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let document_type = map.get_str("$type")?;

        let document_type = data_contract.document_type_for_name(document_type)?;

        let mut identifiers_paths = document_type.identifier_paths().to_owned();

        identifiers_paths.extend(IDENTIFIER_FIELDS.iter().map(|s| s.to_string()));

        let mut binary_paths = document_type.binary_paths().to_owned();

        binary_paths.extend(BINARY_FIELDS.iter().map(|s| s.to_string()));

        map.replace_at_paths(binary_paths.iter(), ReplacementType::BinaryBytes)?;

        map.replace_at_paths(identifiers_paths.iter(), ReplacementType::Identifier)?;
        let document = Self::from_value_map(map, data_contract)?;

        Ok(document)
    }

    #[cfg(feature = "state-transition-value-conversion")]
    fn from_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let map = raw_transition
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Self::from_value_map(map, data_contract)
    }

    #[cfg(feature = "state-transition-value-conversion")]
    fn from_value_map(
        mut map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let version = map.remove_string(STATE_TRANSITION_PROTOCOL_VERSION)?;
        match version.as_str() {
            "0" => Ok(DocumentCreateTransition::V0(
                DocumentCreateTransitionV0::from_value_map(map, data_contract)?,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentMethodV0::hash".to_string(),
                known_versions: vec![0],
                received: version.parse().map_err(|_| {
                    ProtocolError::Generic("received non string version".to_string())
                })?,
            }),
        }
    }

    #[cfg(feature = "state-transition-value-conversion")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }

    #[cfg(feature = "state-transition-value-conversion")]
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            DocumentCreateTransition::V0(v0) => {
                let mut value_map = v0.to_value_map()?;
                value_map.insert(STATE_TRANSITION_PROTOCOL_VERSION.to_owned(), "0".into());
                Ok(value_map)
            }
        }
    }

    #[cfg(feature = "state-transition-json-conversion")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_cleaned_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "state-transition-value-conversion")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map()?.into())
    }
}
