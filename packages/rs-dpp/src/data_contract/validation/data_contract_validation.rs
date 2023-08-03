use itertools::Itertools;
use lazy_static::lazy_static;
use log::trace;
use platform_value::Value;
use serde_json::Value as JsonValue;

use super::{
    multi_validator::{
        self, byte_array_has_no_items_as_parent_validator, pattern_is_valid_regex_validator,
    },
    validate_data_contract_max_depth::validate_data_contract_max_depth,
};

pub const MAX_INDEXED_STRING_PROPERTY_LENGTH: usize = 63;

pub const NOT_ALLOWED_SYSTEM_PROPERTIES: [&str; 1] = ["$id"];
pub const ALLOWED_INDEX_SYSTEM_PROPERTIES: [&str; 3] = ["$ownerId", "$createdAt", "$updatedAt"];
pub const MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH: usize = 255;
pub const MAX_INDEXED_ARRAY_ITEMS: usize = 1024;

lazy_static! {
    pub static ref BASE_DOCUMENT_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("../../../schema/document/documentBase.json")).unwrap();
}

//
// impl DataContract {
//     /// checks the correctness of indices and returns the validation result. The bool flags should be on,
//     /// when further validation should be stopped
//     pub fn validate_index_definitions(
//         &self,
//         document_schema: &JsonValue,
//     ) -> (SimpleConsensusValidationResult, bool) {
//         let mut result = ConsensusValidationResult::default();
//         let mut indices_fingerprints: Vec<String> = vec![];
//
//         for document_type in self.document_types().values() {
//             for index in document_type.index_structure()
//         }
//
//         for index_definition in indices.iter() {
//             let validation_result =
//                 DataContract::validate_no_system_indices(index_definition, document_type);
//             result.merge(validation_result);
//
//             let user_defined_properties = index_definition
//                 .properties
//                 .iter()
//                 .map(|property| &property.name)
//                 .filter(|property_name| {
//                     !ALLOWED_INDEX_SYSTEM_PROPERTIES.contains(&property_name.as_str())
//                 });
//
//             let property_definition_entities: HashMap<&String, Option<&JsonValue>> =
//                 user_defined_properties
//                     .map(|property_name| {
//                         (
//                             property_name,
//                             get_property_definition_by_path(document_schema, property_name).ok(),
//                         )
//                     })
//                     .collect();
//
//             let validation_result = DataContract::validate_not_defined_properties(
//                 &property_definition_entities,
//                 index_definition,
//                 document_type,
//             );
//
//             if !validation_result.is_valid() {
//                 result.merge(validation_result);
//                 // Skip further validation if there are undefined properties
//                 return (result, true);
//             }
//
//             // Validation of property defs
//             for (property_name, maybe_property_definition) in property_definition_entities {
//                 result.merge(DataContract::validate_property_definition(
//                     property_name,
//                     maybe_property_definition,
//                     &document_type,
//                     index_definition,
//                 ));
//             }
//
//             // Make sure that compound unique indices contain all fields
//             if index_definition.unique && index_definition.properties.len() > 1 {
//                 // let required_fields = document_schema
//                 //     .get_schema_required_fields()
//                 //     .unwrap_or_default();
//                 // let all_are_required = index_definition
//                 //     .properties
//                 //     .iter()
//                 //     .map(|property| &property.name)
//                 //     .all(|field| required_fields.contains(&field.as_str()));
//                 //
//                 // let all_are_not_required = index_definition
//                 //     .properties
//                 //     .iter()
//                 //     .map(|property| &property.name)
//                 //     .all(|field| !required_fields.contains(&field.as_str()));
//                 //
//                 // if !all_are_required && !all_are_not_required {
//                 //     result.add_error(BasicError::IndexError(
//                 //         IndexError::InvalidCompoundIndexError(InvalidCompoundIndexError::new(
//                 //             document_type.to_owned(),
//                 //             index_definition.clone(),
//                 //         )),
//                 //     ));
//                 // }
//             }
//         }
//         (result, false)
//     }
// }
//
// impl DataContract {
//     fn validate_property_definition(
//         property_name: &str,
//         maybe_property_definition: Option<&JsonValue>,
//         document_type: &str,
//         index_definition: &Index,
//     ) -> SimpleConsensusValidationResult {
//         let mut result = SimpleConsensusValidationResult::default();
//
//         // we are allowed to use unwrap as we return if some of the properties definitions is None
//         let property_definition = maybe_property_definition.unwrap();
//         let is_byte_array = property_definition.is_type_of_byte_array();
//         let mut invalid_property_type: String = "".to_string();
//
//         if property_definition.is_type_of_object() {
//             invalid_property_type = "object".to_string()
//         }
//
//         // Validate arrays contain scalar values or have the same types
//         // https://github.com/dashevo/platform/blob/ab6391f4b47a970c733e7b81115b44329fbdf993/packages/js-dpp/lib/dataContract/validation/validateDataContractFactory.js#L210
//         if property_definition.is_type_of_array() && !is_byte_array {
//             invalid_property_type = "array".to_string();
//             // const isInvalidPrefixItems = prefixItems
//             //   && (
//             // prefixItems.some((prefixItem) =>
//             // prefixItem.type === 'object' || prefixItem.type === 'array')
//             //     || !prefixItems.every((prefixItem) => prefixItem.type === prefixItems[0].type)
//             //   );
//             //
//             // const isInvalidItemTypes = items.type === 'object' || items.type === 'array';
//             //
//             // if (isInvalidPrefixItems || isInvalidItemTypes) {
//             //   invalidPropertyType = 'array';
//             // }
//         }
//
//         if !invalid_property_type.is_empty() {
//             result.add_error(ConsensusError::BasicError(
//                 BasicError::InvalidIndexPropertyTypeError(InvalidIndexPropertyTypeError::new(
//                     document_type.to_owned(),
//                     index_definition.name.to_owned(),
//                     property_name.to_owned(),
//                     invalid_property_type.to_owned(),
//                 )),
//             ));
//         }
//
//         // https://github.com/dashevo/platform/blob/ab6391f4b47a970c733e7b81115b44329fbdf993/packages/js-dpp/lib/dataContract/validation/validateDataContractFactory.js#L236
//         // Validate sting length inside arrays
//         // if (!invalidPropertyType && propertyType === 'array' && !isByteArray) {
//         //   const isInvalidPrefixItems = prefixItems && prefixItems.some((prefixItem) => (
//         //     prefixItem.type === 'string'
//         //     && (
//         // !prefixItem.maxLength || prefixItem.maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH
//         //     )
//         //   ));
//         //
//         //   const isInvalidItemTypes = items.type === 'string' && (
//         //     !items.maxLength || items.maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH
//         //   );
//         //
//         //   if (isInvalidPrefixItems || isInvalidItemTypes) {
//         //     result.addError(
//         //       new InvalidIndexedPropertyConstraintError(
//         //         documentType,
//         //         indexDefinition,
//         //         propertyName,
//         //         'maxLength',
//         //         `should be less or equal ${MAX_INDEXED_STRING_PROPERTY_LENGTH}`,
//         //       ),
//         //     );
//         //   }
//         // }
//         //
//
//         if invalid_property_type.is_empty() && property_definition.is_type_of_array() {
//             let max_items = property_definition.get_u64("maxItems").ok();
//             let max_limit = if is_byte_array {
//                 MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH
//             } else {
//                 MAX_INDEXED_ARRAY_ITEMS
//             };
//
//             if max_items.is_none() || max_items.unwrap() > max_limit as u64 {
//                 result.add_error(ConsensusError::BasicError(
//                     BasicError::InvalidIndexedPropertyConstraintError(
//                         InvalidIndexedPropertyConstraintError::new(
//                             document_type.to_owned(),
//                             index_definition.name.to_owned(),
//                             property_name.to_owned(),
//                             String::from("maxItems"),
//                             format!("should be less or equal {}", max_limit),
//                         ),
//                     ),
//                 ));
//             }
//         }
//
//         if property_definition.is_type_of_string() {
//             let max_length = property_definition.get_u64("maxLength").ok();
//
//             if max_length.is_none()
//                 || max_length.unwrap() > MAX_INDEXED_STRING_PROPERTY_LENGTH as u64
//             {
//                 result.add_error(ConsensusError::BasicError(
//                     BasicError::InvalidIndexedPropertyConstraintError(
//                         InvalidIndexedPropertyConstraintError::new(
//                             document_type.to_owned(),
//                             index_definition.name.to_owned(),
//                             property_name.to_owned(),
//                             String::from("maxLength"),
//                             format!(
//                                 "should be less or equal than {}",
//                                 MAX_INDEXED_STRING_PROPERTY_LENGTH
//                             ),
//                         ),
//                     ),
//                 ))
//             }
//         }
//
//         result
//     }
// }
//
// impl DataContract {
//     /// checks if properties defined in indices are existing in the contract
//     fn validate_not_defined_properties(
//         properties: &HashMap<&String, Option<&JsonValue>>,
//         index_definition: &Index,
//         document_type: &str,
//     ) -> SimpleConsensusValidationResult {
//         let mut result = SimpleConsensusValidationResult::default();
//         for (property_name, definition) in properties {
//             if definition.is_none() {
//                 result.add_error(ConsensusError::BasicError(
//                     BasicError::UndefinedIndexPropertyError(UndefinedIndexPropertyError::new(
//                         document_type.to_owned(),
//                         index_definition.name.to_owned(),
//                         property_name.to_owned().to_owned(),
//                     )),
//                 ))
//             }
//         }
//         result
//     }
// }
//
// impl DataContract {
//     /// checks if names of indices are not duplicated
//     pub fn validate_index_naming_duplicates(
//         indices: &[Index],
//         document_type: &str,
//     ) -> SimpleConsensusValidationResult {
//         let mut result = SimpleConsensusValidationResult::default();
//         for duplicate_index in indices.iter().map(|i| &i.name).duplicates() {
//             result.add_error(BasicError::DuplicateIndexNameError(
//                 DuplicateIndexNameError::new(document_type.to_owned(), duplicate_index.to_owned()),
//             ))
//         }
//         result
//     }
// }
//
