use crate::data_contract::document_type::reference::DocumentPropertyReference;
use crate::data_contract::document_type::{DocumentPropertyType, DocumentType};
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub mod v0;
impl DocumentType {
    pub(super) fn parse_property_reference(
        data_contract_system_version: u16,
        inner_properties: &BTreeMap<String, &Value>,
        property_type: &DocumentPropertyType,
        platform_version: &PlatformVersion,
    ) -> Result<Option<DocumentPropertyReference>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .class_method_versions
            .parse_property_reference
        {
            None => Ok(None),
            Some(0) => DocumentType::parse_property_reference_v0(
                data_contract_system_version,
                inner_properties,
                property_type,
            ),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "try_from_schema".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::document_type::reference::DocumentPropertyReferenceTarget;
    use assert_matches::assert_matches;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn should_parse_refers_to_on_identifier_property() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identity"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            2,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let reference = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .and_then(|p| p.reference.clone())
            .expect("reference should be present");

        assert_matches!(
            reference.target,
            DocumentPropertyReferenceTarget::IdentityReferenceTarget
        );
        assert!(reference.must_exist);
    }

    #[test]
    fn should_not_parse_refers_to_reference_on_contracts_before_system_version_2() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identity"
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            0,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let reference = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .and_then(|p| p.reference.clone());

        assert_matches!(reference, None);
    }

    #[test]
    fn should_reject_refers_to_on_non_identifier_property() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                    "refersTo": { "type": "identity" }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let err = DocumentType::try_from_schema(
            Identifier::random(),
            2,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("refersTo is only allowed on identifier properties"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn should_parse_refers_to_with_must_exist_false() {
        let platform_version = PlatformVersion::latest();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("config should build");

        let schema = json!({
            "type": "object",
            "properties": {
                "toUserId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0,
                    "refersTo": {
                        "type": "identity",
                        "mustExist": false
                    }
                }
            },
            "required": [],
            "additionalProperties": false
        });

        let value = platform_value::to_value(schema).expect("schema should convert");

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            2,
            config.version(),
            "msg",
            value,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("should parse");

        let reference = document_type
            .as_ref()
            .flattened_properties()
            .get("toUserId")
            .and_then(|p| p.reference.clone())
            .expect("reference should be present");

        assert_matches!(
            reference.target,
            DocumentPropertyReferenceTarget::IdentityReferenceTarget
        );
        assert!(!reference.must_exist);
    }
}
