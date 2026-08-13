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
