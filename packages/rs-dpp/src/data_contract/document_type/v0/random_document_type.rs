// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Random Document types.
//!
//!
//!
#[derive(Clone, Copy, Debug, PartialEq, Encode, Decode)]
pub struct FieldTypeWeights {
    pub string_weight: u16,
    pub float_weight: u16,
    pub integer_weight: u16,
    pub date_weight: u16,
    pub boolean_weight: u16,
    pub byte_array_weight: u16,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct FieldMinMaxBounds {
    pub string_min_len: Range<u16>,
    pub string_has_min_len_chance: f64,
    pub string_max_len: Range<u16>,
    pub string_has_max_len_chance: f64,
    pub integer_min: Range<u16>,
    pub integer_has_min_chance: f64,
    pub integer_max: Range<u16>,
    pub integer_has_max_chance: f64,
    pub float_min: Range<f64>,
    pub float_has_min_chance: f64,
    pub float_max: Range<f64>,
    pub float_has_max_chance: f64,
    pub date_min: i64,
    pub date_max: i64,
    pub byte_array_min_len: Range<u16>,
    pub byte_array_has_min_len_chance: f64,
    pub byte_array_max_len: Range<u16>,
    pub byte_array_has_max_len_chance: f64,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct RandomDocumentTypeParameters {
    pub new_fields_optional_count_range: Range<u16>,
    pub new_fields_required_count_range: Range<u16>,
    pub new_indexes_count_range: Range<u16>,
    pub field_weights: FieldTypeWeights,
    pub field_bounds: FieldMinMaxBounds,
    pub keep_history_chance: f64,
    pub documents_mutable_chance: f64,
}

impl RandomDocumentTypeParameters {
    fn validate_parameters(&self) -> Result<(), ProtocolError> {
        let min_string_len = self.field_bounds.string_min_len.end;
        let max_string_len = self.field_bounds.string_max_len.start;
        if min_string_len > max_string_len {
            return Err(ProtocolError::Generic(
                "String min length range end is greater than max length range start".to_string(),
            ));
        }

        let min_byte_array_len = self.field_bounds.byte_array_min_len.end;
        let max_byte_array_len = self.field_bounds.byte_array_max_len.start;
        if min_byte_array_len > max_byte_array_len {
            return Err(ProtocolError::Generic(
                "Byte array min length range end is greater than max length range start"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::v0::{DocumentTypeV0, StatelessJsonSchemaLazyValidator};
use crate::data_contract::document_type::{
    DocumentProperty, DocumentPropertyType, DocumentType, Index,
};
use crate::identity::SecurityLevel;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::{Identifier, Value};
use rand::rngs::StdRng;
use rand::Rng;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

impl DocumentTypeV0 {
    pub fn random_document_type(
        parameters: RandomDocumentTypeParameters,
        data_contract_id: Identifier,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // Call the validation function at the beginning
        parameters.validate_parameters()?;

        let field_weights = &parameters.field_weights;

        let total_weight = field_weights.string_weight
            + field_weights.float_weight
            + field_weights.integer_weight
            + field_weights.date_weight
            + field_weights.boolean_weight
            + field_weights.byte_array_weight;

        let random_field = |required: bool, rng: &mut StdRng| -> DocumentProperty {
            let random_weight = rng.gen_range(0..total_weight);
            let document_type = if random_weight < field_weights.string_weight {
                let has_min_len = rng.gen_bool(parameters.field_bounds.string_has_min_len_chance);
                let has_max_len = rng.gen_bool(parameters.field_bounds.string_has_max_len_chance);
                let min_len = if has_min_len {
                    Some(rng.gen_range(parameters.field_bounds.string_min_len.clone()))
                } else {
                    None
                };
                let max_len = if has_max_len {
                    Some(rng.gen_range(parameters.field_bounds.string_max_len.clone()))
                } else {
                    None
                };
                DocumentPropertyType::String(min_len, max_len)
            } else if random_weight < field_weights.string_weight + field_weights.integer_weight {
                DocumentPropertyType::Integer
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
            {
                DocumentPropertyType::Number
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
                    + field_weights.date_weight
            {
                DocumentPropertyType::Date
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
                    + field_weights.date_weight
                    + field_weights.boolean_weight
            {
                DocumentPropertyType::Boolean
            } else {
                let has_min_len =
                    rng.gen_bool(parameters.field_bounds.byte_array_has_min_len_chance);
                let has_max_len =
                    rng.gen_bool(parameters.field_bounds.byte_array_has_max_len_chance);
                let min_len = if has_min_len {
                    Some(rng.gen_range(parameters.field_bounds.byte_array_min_len.clone()))
                } else {
                    None
                };
                let max_len = if has_max_len {
                    Some(rng.gen_range(parameters.field_bounds.byte_array_max_len.clone()))
                } else {
                    None
                };

                DocumentPropertyType::ByteArray(min_len, max_len)
            };

            DocumentProperty {
                property_type: document_type,
                required,
            }
        };

        let optional_field_count =
            rng.gen_range(parameters.new_fields_optional_count_range.clone());
        let required_field_count =
            rng.gen_range(parameters.new_fields_required_count_range.clone());

        let mut properties = BTreeMap::new();
        let mut required_fields = BTreeSet::new();

        for _ in 0..optional_field_count {
            let field_name = format!("field_{}", rng.gen::<u16>());
            properties.insert(field_name, random_field(false, rng));
        }

        for _ in 0..required_field_count {
            let field_name = format!("field_{}", rng.gen::<u16>());
            properties.insert(field_name.clone(), random_field(true, rng));
            required_fields.insert(field_name);
        }

        let index_count = rng.gen_range(parameters.new_indexes_count_range.clone());
        let field_names: Vec<String> = properties.keys().cloned().collect();
        let mut indices = Vec::with_capacity(index_count as usize);

        for _ in 0..index_count {
            match Index::random(&field_names, &indices, rng) {
                Ok(index) => indices.push(index),
                Err(_) => break,
            }
        }

        let documents_keep_history = rng.gen_bool(parameters.keep_history_chance);
        let documents_mutable = rng.gen_bool(parameters.documents_mutable_chance);

        let name = format!("doc_type_{}", rng.gen::<u16>());

        let index_structure =
            IndexLevel::try_from_indices(indices.as_slice(), name.as_str(), platform_version)?;
        let (identifier_paths, binary_paths) = DocumentType::find_identifier_and_binary_paths(
            &properties,
            &PlatformVersion::latest()
                .dpp
                .contract_versions
                .document_type_versions,
        )?;

        // TODO: It might not work properly
        Ok(DocumentTypeV0 {
            name,
            schema: Value::Null,
            indices,
            index_structure,
            flattened_properties: properties.clone(),
            properties,
            identifier_paths,
            binary_paths,
            required_fields,
            documents_keep_history,
            documents_mutable,
            data_contract_id,
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
            security_level_requirement: SecurityLevel::HIGH,
            json_schema_validator: StatelessJsonSchemaLazyValidator::new(),
        })
    }
}
