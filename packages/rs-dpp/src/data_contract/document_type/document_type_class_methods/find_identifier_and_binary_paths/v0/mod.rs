use crate::data_contract::document_type::array_field::ArrayFieldType;
use crate::data_contract::document_type::document_field::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::DocumentType;
use std::collections::{BTreeMap, BTreeSet};

impl DocumentType {
    pub(super) fn find_identifier_and_binary_paths_v0(
        properties: &BTreeMap<String, DocumentProperty>,
    ) -> (BTreeSet<String>, BTreeSet<String>) {
        Self::find_identifier_and_binary_paths_inner(properties, "")
    }

    fn find_identifier_and_binary_paths_inner(
        properties: &BTreeMap<String, DocumentProperty>,
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

            match &value.r#type {
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
                DocumentPropertyType::Array(array_field_type) => {
                    let new_path = format!("{}[]", new_path);
                    match array_field_type {
                        ArrayFieldType::Identifier => {
                            identifier_paths.insert(new_path.clone());
                        }
                        ArrayFieldType::ByteArray(_, _) => {
                            binary_paths.insert(new_path.clone());
                        }
                        _ => {}
                    }
                }
                DocumentPropertyType::VariableTypeArray(array_field_types) => {
                    for (i, array_field_type) in array_field_types.iter().enumerate() {
                        let new_path = format!("{}[{}]", new_path, i);
                        match array_field_type {
                            ArrayFieldType::Identifier => {
                                identifier_paths.insert(new_path.clone());
                            }
                            ArrayFieldType::ByteArray(_, _) => {
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
