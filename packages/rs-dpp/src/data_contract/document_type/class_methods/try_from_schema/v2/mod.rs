use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV2Setters;
use crate::data_contract::document_type::class_methods::{
    consensus_or_protocol_data_contract_error, consensus_or_protocol_value_error,
};
use crate::data_contract::document_type::property_names::CREATION_RESTRICTION_GROUP;
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::data_contract::document_type::v2::DocumentTypeV2;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{GroupContractPosition, TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentTypeV2 {
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_schema(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut document_type: DocumentTypeV2 = DocumentTypeV1::try_from_schema(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            validation_operations,
            platform_version,
        )
        .map(Into::into)?;

        let schema_map = document_type.schema.to_map().map_err(|err| {
            consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
                format!("document schema must be an object: {err}"),
            ))
        })?;

        let creation_restriction_group: Option<GroupContractPosition> =
            Value::inner_optional_integer_value(schema_map, CREATION_RESTRICTION_GROUP)
                .map_err(consensus_or_protocol_value_error)?;

        match document_type.creation_restriction_mode {
            CreationRestrictionMode::AnyGroupMember => {
                if creation_restriction_group.is_none() {
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(
                            "creationRestrictionGroup is required when creationRestrictionMode is 3"
                                .to_string(),
                        ),
                    ));
                }
            }
            _ => {
                if creation_restriction_group.is_some() {
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(
                            "creationRestrictionGroup is only allowed when creationRestrictionMode is 3"
                                .to_string(),
                        ),
                    ));
                }
            }
        }

        document_type.set_creation_restriction_group(creation_restriction_group);

        Ok(document_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::DataContractConfig;
    use assert_matches::assert_matches;
    use platform_value::platform_value;
    use std::collections::BTreeMap;

    fn default_config(platform_version: &PlatformVersion) -> DataContractConfig {
        DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config")
    }

    #[test]
    fn should_require_group_for_any_group_member_mode() {
        let platform_version = PlatformVersion::latest();
        let config = default_config(platform_version);
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                }
            },
            "creationRestrictionMode": 3,
            "additionalProperties": false,
        });

        let result = DocumentTypeV2::try_from_schema(
            Identifier::random(),
            1,
            config.version(),
            "testDoc",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        );

        const EXPECTED_ERROR_MSG: &str =
            "creationRestrictionGroup is required when creationRestrictionMode is 3";
        assert_matches!(
            result,
            Err(ProtocolError::ConsensusError(boxed)) => {
                assert_matches!(
                    boxed.as_ref(),
                    ConsensusError::BasicError(
                        BasicError::ContractError(DataContractError::InvalidContractStructure(msg))
                    ) if msg.eq(EXPECTED_ERROR_MSG)
                )
            }
        );
    }

    #[test]
    fn should_forbid_group_when_not_any_group_member_mode() {
        let platform_version = PlatformVersion::latest();
        let config = default_config(platform_version);
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                }
            },
            "creationRestrictionMode": 1,
            "creationRestrictionGroup": 0,
            "additionalProperties": false,
        });

        let result = DocumentTypeV2::try_from_schema(
            Identifier::random(),
            1,
            config.version(),
            "testDoc",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        );

        const EXPECTED_ERROR_MSG: &str =
            "creationRestrictionGroup is only allowed when creationRestrictionMode is 3";
        assert_matches!(
            result,
            Err(ProtocolError::ConsensusError(boxed)) => {
                assert_matches!(
                    boxed.as_ref(),
                    ConsensusError::BasicError(
                        BasicError::ContractError(DataContractError::InvalidContractStructure(msg))
                    ) if msg.eq(EXPECTED_ERROR_MSG)
                )
            }
        );
    }

    #[test]
    fn should_accept_group_when_any_group_member_mode() {
        let platform_version = PlatformVersion::latest();
        let config = default_config(platform_version);
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                }
            },
            "creationRestrictionMode": 3,
            "creationRestrictionGroup": 0,
            "additionalProperties": false,
        });

        let result = DocumentTypeV2::try_from_schema(
            Identifier::random(),
            1,
            config.version(),
            "testDoc",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        );

        assert!(result.is_ok());
    }
}
