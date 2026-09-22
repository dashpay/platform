use crate::consensus::basic::data_contract::DocumentTypesAreMissingError;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentType,
};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{DocumentName, TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentType {
    /// Generation 2: generation 1 (contracts with only tokens) plus the check that an
    /// `identityPublicKey` reference's `keyRequirements.boundTo` names a document type of the
    /// same contract. A document type's parse sees only its own schema, so the check runs here,
    /// once every document type of the contract is parsed. It holds the reference's promise that
    /// the write-time check never needs a second contract fetch: the bound the key must carry
    /// names the declaring contract and one of its own document types.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract) fn create_document_types_from_document_schemas_v2(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        document_schemas: BTreeMap<DocumentName, Value>,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        has_tokens: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
        let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();

        if document_schemas.is_empty() && !has_tokens {
            return Err(consensus_or_protocol_data_contract_error(
                DocumentTypesAreMissingError::new(data_contract_id).into(),
            ));
        }

        for (name, schema) in document_schemas.into_iter() {
            let document_type = match platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .structure_version
            {
                0 => DocumentType::try_from_schema(
                    data_contract_id,
                    data_contract_system_version,
                    contract_config_version,
                    &name,
                    schema,
                    schema_defs,
                    token_configurations,
                    data_contact_config,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?,
                version => {
                    return Err(ProtocolError::UnknownVersionMismatch {
                        method: "get_document_types_from_value_array_v0 inner document type"
                            .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })
                }
            };

            contract_document_types.insert(name.to_string(), document_type);
        }

        for (name, document_type) in &contract_document_types {
            for (path, property) in document_type.as_ref().flattened_properties() {
                let DocumentPropertyType::IdentifierWithReference(
                    DocumentPropertyReferenceTarget::IdentityPublicKey {
                        key_requirements, ..
                    },
                ) = &property.property_type
                else {
                    continue;
                };
                let Some(bound_to) = &key_requirements.bound_to else {
                    continue;
                };
                if !contract_document_types.contains_key(bound_to) {
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(format!(
                            "{name}.{path} refersTo keyRequirements boundTo {bound_to:?} names no document type of this contract"
                        )),
                    ));
                }
            }
        }

        Ok(contract_document_types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::data_contract::document_type::IdentityKeyReferenceRequirements;
    use crate::data_contract::DataContract;
    use crate::identity::Purpose;
    use crate::serialization::{
        PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
        PlatformSerializableWithPlatformVersion,
    };
    use platform_value::platform_value;
    use platform_value::string_encoding::Encoding;
    use std::ops::Deref;

    /// A contract with a `joinRequest` type whose `recipientId` references an identity key
    /// with `refers_to`, and a `submittedCharter` type for a bound to name.
    fn contract_value(refers_to: Value) -> Value {
        platform_value!({
            "$formatVersion": "1",
            "id": Identifier::from_string("4Bqs6itzfoDXzmgQibYZQABbqYsXmawVf7SKe3mKDQVd", Encoding::Base58).expect("a valid id"),
            "ownerId": Identifier::from_string("2b994p95akyNFKtkDnDvBRUotDbkH54MHwGbhQLr5gcU", Encoding::Base58).expect("a valid id"),
            "version": 1,
            "documentSchemas": {
                "joinRequest": {
                    "type": "object",
                    "properties": {
                        "recipientId": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "contentMediaType": "application/x.dash.dpp.identifier",
                            "position": 0,
                            "refersTo": refers_to
                        },
                        "recipientKeyId": {
                            "type": "integer",
                            "minimum": 0,
                            "position": 1
                        }
                    },
                    "required": [],
                    "additionalProperties": false
                },
                "submittedCharter": {
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "maxLength": 64,
                            "position": 0
                        }
                    },
                    "required": [],
                    "additionalProperties": false
                }
            }
        })
    }

    fn recipient_id_target(contract: &DataContract) -> DocumentPropertyType {
        contract
            .document_type_for_name("joinRequest")
            .expect("the joinRequest document type")
            .flattened_properties()
            .get("recipientId")
            .map(|p| p.property_type.clone())
            .expect("the recipientId property")
    }

    #[test]
    fn should_accept_bound_to_naming_a_document_type_of_the_contract() {
        let platform_version = PlatformVersion::latest();

        for bound_to in ["submittedCharter", "joinRequest"] {
            let contract = DataContract::from_value(
                contract_value(platform_value!({
                    "type": "identityPublicKey",
                    "keyIdProperty": "recipientKeyId",
                    "keyRequirements": { "purpose": "decryption", "boundTo": bound_to }
                })),
                true,
                platform_version,
            )
            .expect("the contract should parse");

            assert_eq!(
                recipient_id_target(&contract),
                DocumentPropertyType::IdentifierWithReference(
                    DocumentPropertyReferenceTarget::IdentityPublicKey {
                        key_id_property: "recipientKeyId".to_string(),
                        key_requirements: IdentityKeyReferenceRequirements {
                            purpose: Some(Purpose::DECRYPTION),
                            bound_to: Some(bound_to.to_string()),
                        },
                    }
                )
            );
        }
    }

    #[test]
    fn should_reject_bound_to_naming_a_document_type_the_contract_does_not_have() {
        let platform_version = PlatformVersion::latest();

        for full_validation in [true, false] {
            let error = DataContract::from_value(
                contract_value(platform_value!({
                    "type": "identityPublicKey",
                    "keyIdProperty": "recipientKeyId",
                    "keyRequirements": { "purpose": "decryption", "boundTo": "electedCharter" }
                })),
                full_validation,
                platform_version,
            )
            .expect_err("the contract should be refused");

            let ProtocolError::ConsensusError(consensus_error) = &error else {
                panic!("expected a consensus error, got {error}");
            };
            let ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(message),
            )) = consensus_error.deref()
            else {
                panic!("expected an invalid contract structure error, got {consensus_error}");
            };
            assert_eq!(
                message,
                "joinRequest.recipientId refersTo keyRequirements boundTo \"electedCharter\" names no document type of this contract"
            );
        }
    }

    #[test]
    fn should_refuse_key_requirements_before_protocol_version_14_and_accept_them_at_it() {
        let value = contract_value(platform_value!({
            "type": "identityPublicKey",
            "keyIdProperty": "recipientKeyId",
            "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" }
        }));

        // The meta-schema of protocol version 13 knows no `refersTo` at all
        let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 exists");
        DataContract::from_value(value.clone(), true, platform_version_13)
            .expect_err("a contract with keyRequirements should be refused before version 14");

        let contract = DataContract::from_value(value, true, PlatformVersion::latest())
            .expect("a contract with keyRequirements should parse at version 14");
        assert!(matches!(
            recipient_id_target(&contract),
            DocumentPropertyType::IdentifierWithReference(
                DocumentPropertyReferenceTarget::IdentityPublicKey { key_requirements, .. }
            ) if key_requirements.purpose == Some(Purpose::DECRYPTION)
                && key_requirements.bound_to.as_deref() == Some("submittedCharter")
        ));
    }

    /// The requirements ride on the schema, which is what the contract serializes, so a
    /// contract with them and one without both come back as parsed, and the one without
    /// serializes exactly as it did before the keyword existed.
    #[test]
    fn should_round_trip_key_requirements_through_platform_serialization() {
        let platform_version = PlatformVersion::latest();

        for refers_to in [
            platform_value!({ "type": "identityPublicKey", "keyIdProperty": "recipientKeyId" }),
            platform_value!({
                "type": "identityPublicKey",
                "keyIdProperty": "recipientKeyId",
                "keyRequirements": { "purpose": "decryption", "boundTo": "submittedCharter" }
            }),
        ] {
            let contract =
                DataContract::from_value(contract_value(refers_to), true, platform_version)
                    .expect("the contract should parse");

            let bytes = contract
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("the contract should serialize");
            let deserialized = DataContract::versioned_deserialize_untrusted(
                bytes.as_slice(),
                false,
                platform_version,
            )
            .expect("the contract should deserialize");

            assert_eq!(
                recipient_id_target(&deserialized),
                recipient_id_target(&contract)
            );
            assert_eq!(
                deserialized
                    .serialize_to_bytes_with_platform_version(platform_version)
                    .expect("the contract should serialize again"),
                bytes
            );
        }
    }
}
