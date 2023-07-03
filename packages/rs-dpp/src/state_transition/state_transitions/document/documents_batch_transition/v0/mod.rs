use crate::data_contract::DataContract;
use crate::document::document_transition::document_base_transition::JsonValue;
use crate::document::document_transition::DocumentTransitionObjectLike;
use crate::identity::KeyID;
use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::state_transition::StateTransitionType;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use anyhow::{anyhow, Context};
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapReplacementPathHelper,
};
use platform_value::string_encoding::Encoding;
use platform_value::{BinaryData, Identifier, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(
    Debug,
    Encode,
    Decode,
    Clone,
    PartialEq,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
)]
#[platform_error_type(ProtocolError)]
pub struct DocumentsBatchTransitionV0 {
    pub owner_id: Identifier,
    pub transitions: Vec<DocumentTransition>,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: Option<KeyID>,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: Option<BinaryData>,
}

impl Default for DocumentsBatchTransitionV0 {
    fn default() -> Self {
        DocumentsBatchTransitionV0 {
            owner_id: Identifier::default(),
            transitions: vec![],
            signature_public_key_id: None,
            signature: None,
        }
    }
}

impl DocumentsBatchTransitionV0 {
    #[cfg(feature = "json-object")]
    pub fn from_json_object(
        json_value: JsonValue,
        data_contracts: Vec<DataContract>,
    ) -> Result<Self, ProtocolError> {
        let mut json_value = json_value;

        let maybe_signature = json_value.get_string(super::property_names::SIGNATURE).ok();
        let signature = if let Some(signature) = maybe_signature {
            Some(BinaryData(
                base64::decode(signature).context("signature exists but isn't valid base64")?,
            ))
        } else {
            None
        };

        let mut batch_transitions = DocumentsBatchTransition {
            feature_version: json_value
                .get_u64(super::property_names::STATE_TRANSITION_PROTOCOL_VERSION)
                // js-dpp allows `protocolVersion` to be undefined
                .unwrap_or(LATEST_VERSION as u64) as u16,
            signature,
            signature_public_key_id: json_value
                .get_u64(super::property_names::SIGNATURE_PUBLIC_KEY_ID)
                .ok()
                .map(|v| v as KeyID),
            owner_id: Identifier::from_string(
                json_value.get_string(super::property_names::OWNER_ID)?,
                Encoding::Base58,
            )?,
            ..Default::default()
        };

        let mut document_transitions: Vec<DocumentTransition> = vec![];
        let maybe_transitions = json_value.remove(super::property_names::TRANSITIONS);
        if let Ok(JsonValue::Array(json_transitions)) = maybe_transitions {
            let data_contracts_map: HashMap<Vec<u8>, DataContract> = data_contracts
                .into_iter()
                .map(|dc| (dc.id.as_bytes().to_vec(), dc))
                .collect();

            for json_transition in json_transitions {
                let id = Identifier::from_string(
                    json_transition.get_string(super::property_names::DATA_CONTRACT_ID)?,
                    Encoding::Base58,
                )?;
                let data_contract =
                    data_contracts_map
                        .get(&id.as_bytes().to_vec())
                        .ok_or_else(|| {
                            anyhow!(
                                "Data Contract doesn't exists for Transition: {:?}",
                                json_transition
                            )
                        })?;
                let document_transition =
                    DocumentTransition::from_json_object(json_transition, data_contract.clone())?;
                document_transitions.push(document_transition);
            }
        }

        batch_transitions.transitions = document_transitions;
        Ok(batch_transitions)
    }

    /// creates the instance of [`DocumentsBatchTransition`] from raw object
    pub fn from_raw_object_with_contracts(
        raw_object: Value,
        data_contracts: Vec<DataContract>,
    ) -> Result<Self, ProtocolError> {
        let map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Self::from_value_map(map, data_contracts)
    }

    /// creates the instance of [`DocumentsBatchTransition`] from a value map
    pub fn from_value_map(
        mut map: BTreeMap<String, Value>,
        data_contracts: Vec<DataContract>,
    ) -> Result<Self, ProtocolError> {
        let mut batch_transitions = DocumentsBatchTransition {
            feature_version: map
                .get_integer(super::property_names::PROTOCOL_VERSION)
                // js-dpp allows `protocolVersion` to be undefined
                .unwrap_or(LATEST_VERSION as u64) as u16,
            signature: map
                .get_optional_binary_data(super::property_names::SIGNATURE)
                .map_err(ProtocolError::ValueError)?,
            signature_public_key_id: map
                .get_optional_integer(super::property_names::SIGNATURE_PUBLIC_KEY_ID)
                .map_err(ProtocolError::ValueError)?,
            owner_id: Identifier::from(
                map.get_hash256_bytes(super::property_names::OWNER_ID)
                    .map_err(ProtocolError::ValueError)?,
            ),
            ..Default::default()
        };

        let mut document_transitions: Vec<DocumentTransition> = vec![];
        let maybe_transitions = map.remove(super::property_names::TRANSITIONS);
        if let Some(Value::Array(raw_transitions)) = maybe_transitions {
            let data_contracts_map: HashMap<Vec<u8>, DataContract> = data_contracts
                .into_iter()
                .map(|dc| (dc.id.as_bytes().to_vec(), dc))
                .collect();

            for raw_transition in raw_transitions {
                let mut raw_transition_map = raw_transition
                    .into_btree_string_map()
                    .map_err(ProtocolError::ValueError)?;
                let data_contract_id = raw_transition_map
                    .get_hash256_bytes(super::property_names::DATA_CONTRACT_ID)?;
                let document_type =
                    raw_transition_map.get_str(super::property_names::DOCUMENT_TYPE)?;
                let data_contract = data_contracts_map
                    .get(data_contract_id.as_slice())
                    .ok_or_else(|| {
                        anyhow!(
                            "Data Contract doesn't exists for Transition: {:?}",
                            raw_transition_map
                        )
                    })?;

                //Because we don't know how the json came in we need to sanitize it
                let (identifiers, binary_paths): (Vec<_>, Vec<_>) =
                    data_contract.get_identifiers_and_binary_paths_owned(document_type)?;

                raw_transition_map
                    .replace_at_paths(
                        identifiers.into_iter().chain(
                            document_base_transition::IDENTIFIER_FIELDS
                                .iter()
                                .map(|a| a.to_string()),
                        ),
                        ReplacementType::Identifier,
                    )
                    .map_err(ProtocolError::ValueError)?;
                raw_transition_map
                    .replace_at_paths(
                        binary_paths.into_iter().chain(
                            document_create_transition::BINARY_FIELDS
                                .iter()
                                .map(|a| a.to_string()),
                        ),
                        ReplacementType::BinaryBytes,
                    )
                    .map_err(ProtocolError::ValueError)?;

                let document_transition =
                    DocumentTransition::from_value_map(raw_transition_map, data_contract.clone())?;
                document_transitions.push(document_transition);
            }
        }

        batch_transitions.transitions = document_transitions;
        Ok(batch_transitions)
    }

    pub fn get_transitions(&self) -> &Vec<DocumentTransition> {
        &self.transitions
    }

    pub fn get_transitions_slice(&self) -> &[DocumentTransition] {
        self.transitions.as_slice()
    }

    pub fn clean_value(value: &mut Value) -> Result<(), platform_value::Error> {
        value.replace_at_paths(IDENTIFIER_FIELDS, ReplacementType::Identifier)?;
        value.replace_integer_type_at_paths(U32_FIELDS, IntegerReplacementType::U32)?;
        Ok(())
    }
}

impl StateTransitionIdentitySigned for DocumentsBatchTransition {
    fn get_owner_id(&self) -> &Identifier {
        &self.owner_id
    }

    fn get_security_level_requirement(&self) -> Vec<crate::identity::SecurityLevel> {
        // Step 1: Get all document types for the ST
        // Step 2: Get document schema for every type
        // If schema has security level, use that, if not, use the default security level
        // Find the highest level (lowest int value) of all documents - the ST's signature
        // requirement is the highest level across all documents affected by the ST./
        let mut highest_security_level = SecurityLevel::lowest_level();

        for transition in self.transitions.iter() {
            let document_type = &transition.base().document_type_name;
            let data_contract = &transition.base().data_contract;
            let maybe_document_schema = data_contract.get_document_schema(document_type);

            if let Ok(document_schema) = maybe_document_schema {
                let document_security_level =
                    get_security_level_requirement(document_schema, DEFAULT_SECURITY_LEVEL);

                // lower enum enum representation means higher in security
                if document_security_level < highest_security_level {
                    highest_security_level = document_security_level
                }
            }
        }
        if highest_security_level == SecurityLevel::MASTER {
            vec![SecurityLevel::MASTER]
        } else {
            (SecurityLevel::CRITICAL as u8..=highest_security_level as u8)
                .map(|security_level| SecurityLevel::try_from(security_level).unwrap())
                .collect()
        }
    }

    fn get_signature_public_key_id(&self) -> Option<KeyID> {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: KeyID) {
        self.signature_public_key_id = Some(key_id);
    }
}

impl DocumentsBatchTransition {
    fn to_value(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        Ok(self.to_value_map(skip_signature)?.into())
    }

    fn to_value_map(&self, skip_signature: bool) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map = BTreeMap::new();
        map.insert(
            super::property_names::PROTOCOL_VERSION.to_string(),
            Value::U16(self.feature_version),
        );
        map.insert(
            super::property_names::TRANSITION_TYPE.to_string(),
            Value::U8(self.transition_type as u8),
        );
        map.insert(
            super::property_names::OWNER_ID.to_string(),
            Value::Identifier(self.owner_id.to_buffer()),
        );

        if !skip_signature {
            if let Some(signature) = self.signature.as_ref() {
                map.insert(
                    super::property_names::SIGNATURE.to_string(),
                    Value::Bytes(signature.to_vec()),
                );
            }
            if let Some(signature_key_id) = self.signature_public_key_id {
                map.insert(
                    super::property_names::SIGNATURE.to_string(),
                    Value::U32(signature_key_id),
                );
            }
        }
        let mut transitions = vec![];
        for transition in self.transitions.iter() {
            transitions.push(transition.to_object()?)
        }
        map.insert(
            super::property_names::TRANSITIONS.to_string(),
            Value::Array(transitions),
        );

        Ok(map)
    }
}

impl StateTransitionConvert for DocumentsBatchTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![super::property_names::SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![super::property_names::OWNER_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![
            super::property_names::SIGNATURE,
            super::property_names::SIGNATURE_PUBLIC_KEY_ID,
        ]
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        self.to_object(skip_signature)
            .and_then(|value| value.try_into().map_err(ProtocolError::ValueError))
    }

    fn to_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            for path in Self::signature_property_paths() {
                let _ = object.remove_values_matching_path(path);
            }
        }
        let mut transitions = vec![];
        for transition in self.transitions.iter() {
            transitions.push(transition.to_object()?)
        }
        object.insert(
            String::from(super::property_names::TRANSITIONS),
            Value::Array(transitions),
        )?;

        Ok(object)
    }

    #[cfg(feature = "cbor")]
    fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut result_buf = self.feature_version.encode_var_vec();
        let value: CborValue = self.to_object(skip_signature)?.try_into()?;

        let map = CborValue::serialized(&value)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        let mut canonical_map: CborCanonicalMap = map.try_into()?;
        canonical_map.remove(super::property_names::PROTOCOL_VERSION);

        // Replace binary fields individually for every transition using respective data contract
        if let Some(CborValue::Array(ref mut transitions)) = canonical_map.get_mut(
            &CborValue::Text(super::property_names::TRANSITIONS.to_string()),
        ) {
            for (i, cbor_transition) in transitions.iter_mut().enumerate() {
                let transition = self
                    .transitions
                    .get(i)
                    .context(format!("transition with index {} doesn't exist", i))?;

                let (identifier_properties, binary_properties) = transition
                    .base()
                    .data_contract
                    .get_identifiers_and_binary_paths(
                        &self.transitions[i].base().document_type_name,
                    )?;

                if transition.get_updated_at().is_none() {
                    cbor_transition.remove("$updatedAt");
                }

                cbor_transition.replace_paths(
                    identifier_properties
                        .into_iter()
                        .chain(binary_properties)
                        .chain(document_base_transition::IDENTIFIER_FIELDS)
                        .chain(document_create_transition::BINARY_FIELDS),
                    FieldType::ArrayInt,
                    FieldType::Bytes,
                );
            }
        }

        canonical_map.replace_paths(
            Self::binary_property_paths()
                .into_iter()
                .chain(Self::identifiers_property_paths()),
            FieldType::ArrayInt,
            FieldType::Bytes,
        );

        if !skip_signature {
            if self.signature.is_none() {
                canonical_map.insert(super::property_names::SIGNATURE, CborValue::Null)
            }
            if self.signature_public_key_id.is_none() {
                canonical_map.insert(
                    super::property_names::SIGNATURE_PUBLIC_KEY_ID,
                    CborValue::Null,
                )
            }
        }

        canonical_map.sort_canonical();

        let mut buffer = canonical_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        result_buf.append(&mut buffer);

        Ok(result_buf)
    }

    fn to_cleaned_object(&self, skip_signature: bool) -> Result<Value, ProtocolError> {
        let mut object: Value = platform_value::to_value(self)?;
        if skip_signature {
            for path in Self::signature_property_paths() {
                let _ = object.remove_values_matching_path(path);
            }
        }
        let mut transitions = vec![];
        for transition in self.transitions.iter() {
            transitions.push(transition.to_cleaned_object()?)
        }
        object.insert(
            String::from(super::property_names::TRANSITIONS),
            Value::Array(transitions),
        )?;

        Ok(object)
    }
}

impl StateTransitionLike for DocumentsBatchTransition {
    fn modified_data_ids(&self) -> Vec<Identifier> {
        self.transitions.iter().map(|t| t.base().id).collect()
    }

    fn state_transition_protocol_version(&self) -> FeatureVersion {
        self.feature_version
    }

    fn signature(&self) -> &BinaryData {
        if let Some(ref signature) = self.signature {
            signature
        } else {
            // TODO This is temporary solution to not break the `get_signature()` method
            // TODO for other transitions
            todo!()
        }
    }

    fn state_transition_type(&self) -> StateTransitionType {
        self.transition_type
    }

    fn set_signature(&mut self, signature: BinaryData) {
        self.signature = Some(signature);
    }
    fn set_signature_bytes(&mut self, signature: Vec<u8>) {
        self.signature = Some(BinaryData::new(signature));
    }
}
