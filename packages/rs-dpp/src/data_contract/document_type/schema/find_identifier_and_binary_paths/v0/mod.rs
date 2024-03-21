use crate::data_contract::document_type::array::ArrayItemType;
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::v0::DocumentTypeV0;

use indexmap::IndexMap;
use std::collections::BTreeSet;

impl DocumentTypeV0 {
    #[inline(always)]
    pub(super) fn find_identifier_and_binary_paths_v0(
        properties: &IndexMap<String, DocumentProperty>,
    ) -> (BTreeSet<String>, BTreeSet<String>) {
        Self::find_identifier_and_binary_paths_inner(properties, "")
    }

    #[inline(always)]
    fn find_identifier_and_binary_paths_inner(
        properties: &IndexMap<String, DocumentProperty>,
        current_path: &str,
    ) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut identifier_paths = BTreeSet::new();
        let mut binary_paths = BTreeSet::new();

        for (key, value) in properties.iter() {
            let new_path = if current_path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", current_path, key)
            };

            match &value.property_type {
                DocumentPropertyType::Identifier => {
                    identifier_paths.insert(new_path);
                }
                DocumentPropertyType::ByteArray(_, _) => {
                    binary_paths.insert(new_path);
                }
                DocumentPropertyType::Object(inner_properties) => {
                    let (inner_identifier_paths, inner_binary_paths) =
                        Self::find_identifier_and_binary_paths_inner(inner_properties, &new_path);

                    identifier_paths.extend(inner_identifier_paths);
                    binary_paths.extend(inner_binary_paths);
                }
                DocumentPropertyType::Array(item_type) => {
                    let new_path = format!("{}[]", new_path);
                    match item_type {
                        ArrayItemType::Identifier => {
                            identifier_paths.insert(new_path.clone());
                        }
                        ArrayItemType::ByteArray(_, _) => {
                            binary_paths.insert(new_path.clone());
                        }
                        _ => {}
                    }
                }
                DocumentPropertyType::VariableTypeArray(item_types) => {
                    for (i, array_field_type) in item_types.iter().enumerate() {
                        let new_path = format!("{}[{}]", new_path, i);
                        match array_field_type {
                            ArrayItemType::Identifier => {
                                identifier_paths.insert(new_path.clone());
                            }
                            ArrayItemType::ByteArray(_, _) => {
                                binary_paths.insert(new_path.clone());
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        (identifier_paths, binary_paths)
    }
}
