use std::collections::BTreeMap;

use indexmap::IndexMap;

use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::data_contract::document_type::DocumentType;

/// A `requiredSince` annotation may never exceed the version of the contract
/// carrying it — requiredness cannot be pre-scheduled at a future version.
/// Runs over the *parsed* properties, the members of objects included, so
/// annotations reached through `$ref` are covered. Called wherever document
/// types are built from a contract's serialized form (creates, updates, and
/// disk loads all pass through there); a no-op for every contract predating
/// the keyword, since their properties carry no annotation. Only protocol
/// version 15 parses one on an object's member, so walking the members finds
/// nothing at earlier versions.
///
/// The failure is the dedicated consensus error, because the input is
/// untrusted schema data: state-transition processing must classify it as
/// consensus-invalid (nonce bump), never as an execution error. Callers map
/// it through
/// [`class_methods::consensus_or_protocol_required_fields_error`](crate::data_contract::document_type::class_methods).
pub(crate) fn validate_required_since_within_contract_version(
    document_types: &BTreeMap<String, DocumentType>,
    contract_version: u32,
) -> Result<(), DataContractInvalidRequiredFieldsUpdateError> {
    for (document_type_name, document_type) in document_types {
        if let Some((property_name, required_since)) =
            first_required_since_above(document_type.as_ref().properties(), None, contract_version)
        {
            return Err(DataContractInvalidRequiredFieldsUpdateError::new(
                document_type_name.clone(),
                format!(
                    "property '{property_name}' carries requiredSince {required_since} which exceeds the contract version {contract_version}"
                ),
            ));
        }
    }
    Ok(())
}

/// The first property of `properties`, at any depth, whose `requiredSince` exceeds
/// `contract_version`, with its dotted path below `prefix`.
fn first_required_since_above(
    properties: &IndexMap<String, DocumentProperty>,
    prefix: Option<&str>,
    contract_version: u32,
) -> Option<(String, u32)> {
    properties.iter().find_map(|(name, property)| {
        let path = prefix.map_or_else(|| name.clone(), |prefix| format!("{prefix}.{name}"));
        match (property.required_since, &property.property_type) {
            (Some(required_since), _) if required_since > contract_version => {
                Some((path, required_since))
            }
            (_, DocumentPropertyType::Object(members)) => {
                first_required_since_above(members, Some(&path), contract_version)
            }
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use platform_value::{platform_value, Identifier};
    use platform_version::version::PlatformVersion;

    /// A type whose object `meta` holds `x`, required from `required_since`.
    fn document_types(required_since: u32) -> BTreeMap<String, DocumentType> {
        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("expected a default config");
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "meta": {
                    "type": "object",
                    "position": 0,
                    "properties": {
                        "x": {"type": "string", "position": 0, "maxLength": 60_u32, "requiredSince": required_since},
                    },
                    "required": ["x"],
                    "additionalProperties": false,
                },
            },
            "additionalProperties": false,
        });
        let document_type = DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "note",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the document type to parse");
        BTreeMap::from([("note".to_string(), document_type)])
    }

    #[test]
    fn should_refuse_a_member_of_an_object_required_from_a_later_version() {
        let error = validate_required_since_within_contract_version(&document_types(3), 2)
            .expect_err("a member may not be required from a version the contract has not reached");
        assert!(error.details().contains("meta.x"), "{}", error.details());

        assert!(validate_required_since_within_contract_version(&document_types(2), 2).is_ok());
    }
}
