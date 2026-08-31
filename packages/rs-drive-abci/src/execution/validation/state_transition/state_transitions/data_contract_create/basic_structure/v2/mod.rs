use crate::error::Error;
use dpp::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use dpp::dashcore::Network;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use super::v1::DataContractCreateStateTransitionBasicStructureValidationV1;

const PROPERTIES: &str = "properties";
const REQUIRED_SINCE: &str = "requiredSince";

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionBasicStructureValidationV2
{
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreateStateTransitionBasicStructureValidationV2 for DataContractCreateTransition {
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // First run all v1 (and transitively v0) validations
        let v1_result = self.validate_basic_structure_v1(network_type, platform_version)?;
        if !v1_result.is_valid() {
            return Ok(v1_result);
        }

        // `requiredSince` names the contract version a property is required
        // from. A freshly created contract is version 1, so the only value
        // that names an existing version is 1 (which is equivalent to plain
        // membership in `required`). Later values would pre-schedule
        // requiredness at a future version — coherent for the wire format,
        // but banned: requiredness changes must arrive with the update that
        // creates the version they name.
        //
        // This raw-JSON scan is an early, cheap rejection only — it cannot
        // see an annotation reached through a `$defs` `$ref`. The
        // authoritative enforcement is
        // `validate_required_since_within_contract_version` in dpp, which
        // runs on the *parsed* properties (references resolved) whenever the
        // contract is built from its serialized form, including this
        // transition's transform into action.
        for (document_type_name, schema) in self.data_contract().document_schemas() {
            let Some(properties) = schema
                .get_optional_value(PROPERTIES)
                .ok()
                .flatten()
                .and_then(|properties| properties.as_map())
            else {
                continue;
            };

            for (property_name, property_schema) in properties {
                let Some(required_since) = property_schema
                    .as_map()
                    .and_then(|map| {
                        map.iter()
                            .find(|(key, _)| key.as_text() == Some(REQUIRED_SINCE))
                    })
                    .and_then(|(_, value)| value.as_integer::<u32>())
                else {
                    continue;
                };

                if required_since != 1 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DataContractInvalidRequiredFieldsUpdateError::new(
                            document_type_name.clone(),
                            format!(
                                "property '{}' of a newly created contract cannot carry requiredSince {} — a fresh contract is version 1",
                                property_name.as_text().unwrap_or_default(),
                                required_since
                            ),
                        )
                        .into(),
                    ));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::platform_value::platform_value;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;

    fn create_transition_with_required_since(
        required_since: u32,
    ) -> (DataContractCreateTransition, &'static PlatformVersion) {
        let platform_version = PlatformVersion::latest();
        let identity_nonce = IdentityNonce::default();

        let data_contract =
            get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                .data_contract_owned();

        let mut data_contract_for_serialization: dpp::data_contract::serialized_version::DataContractInSerializationFormat = data_contract
            .try_into_platform_versioned(platform_version)
            .expect("failed to convert data contract");

        data_contract_for_serialization
            .document_schemas_mut()
            .insert(
                "note".to_string(),
                platform_value!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "position": 0,
                            "maxLength": 60,
                            "requiredSince": required_since,
                        }
                    },
                    "required": ["message"],
                    "additionalProperties": false
                }),
            );

        let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
            data_contract: data_contract_for_serialization,
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        (transition, platform_version)
    }

    #[test]
    fn should_accept_required_since_of_one_on_a_new_contract() {
        let (transition, platform_version) = create_transition_with_required_since(1);

        let result = transition
            .validate_basic_structure_v2(Network::Testnet, platform_version)
            .expect("failed to validate basic structure");

        assert!(
            result.is_valid(),
            "requiredSince 1 on a fresh contract is equivalent to plain \
             required and must be accepted, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_required_since_above_one_on_a_new_contract() {
        let (transition, platform_version) = create_transition_with_required_since(2);

        let result = transition
            .validate_basic_structure_v2(Network::Testnet, platform_version)
            .expect("failed to validate basic structure");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
            )] if e.details().contains("cannot carry requiredSince 2")
        );
    }
}
