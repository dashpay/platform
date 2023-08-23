use crate::state_transition::data_contract_update_transition::{
    SIGNATURE, SIGNATURE_PUBLIC_KEY_ID,
};
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::documents_batch_transition::fields::property_names::{
    STATE_TRANSITION_PROTOCOL_VERSION, TRANSITIONS,
};
use crate::state_transition::documents_batch_transition::{
    document_base_transition, document_create_transition, DocumentsBatchTransitionV0,
};
use crate::state_transition::{FeatureVersioned, StateTransitionCborConvert, StateTransitionFieldTypes, StateTransitionValueConvert};
use crate::util::cbor_value::{CborCanonicalMap, FieldType, ReplacePaths, ValuesCollection};
use crate::ProtocolError;
use anyhow::Context;
use ciborium::Value as CborValue;
use std::convert::TryInto;
use integer_encoding::VarInt;

impl<'a> StateTransitionCborConvert<'a> for DocumentsBatchTransitionV0 {
    fn to_cbor_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut result_buf = self.feature_version().encode_var_vec();
        let value: CborValue = self.to_object(skip_signature)?.try_into()?;

        let map = CborValue::serialized(&value)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        let mut canonical_map: CborCanonicalMap = map.try_into()?;
        canonical_map.remove(STATE_TRANSITION_PROTOCOL_VERSION);

        // Replace binary fields individually for every transition using respective data contract
        if let Some(CborValue::Array(ref mut transitions)) =
            canonical_map.get_mut(&CborValue::Text(TRANSITIONS.to_string()))
        {
            for (i, cbor_transition) in transitions.iter_mut().enumerate() {
                let transition = self
                    .transitions
                    .get(i)
                    .context(format!("transition with index {} doesn't exist", i))?;

                let mut identifiers_paths = document_type.identifier_paths().to_owned();

                identifiers_paths.extend(IDENTIFIER_FIELDS.iter().map(|s| s.to_string()));

                let mut binary_paths = document_type.binary_paths().to_owned();

                binary_paths.extend(BINARY_FIELDS.iter().map(|s| s.to_string()));

                let (identifier_properties, binary_properties) = transition
                    .base()
                    .data_contract()
                    .get_identifiers_and_binary_paths(
                        &self.transitions[i].base().document_type_name,
                    )?;

                if transition.updated_at().is_none() {
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
                canonical_map.insert(SIGNATURE, CborValue::Null)
            }
            if self.signature_public_key_id.is_none() {
                canonical_map.insert(SIGNATURE_PUBLIC_KEY_ID, CborValue::Null)
            }
        }

        canonical_map.sort_canonical();

        let mut buffer = canonical_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        result_buf.append(&mut buffer);

        Ok(result_buf)
    }
}
