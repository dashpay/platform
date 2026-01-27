mod v1;

pub(in crate::state_transition::state_transitions::contract) use v1::update_contract_cost_from_fields;

use crate::fee::Credits;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::methods::registration_cost::registration_cost_from_fields;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransition {
    /// Returns the update cost of the data contract update based on the current platform version.
    ///
    /// This calculates the cost for updating existing document schemas (not new items).
    /// For V0 transitions, this uses the registration cost logic (charging for all schemas/tokens/keywords).
    /// For V1 transitions, this calculates the cost based on updated_document_schemas only.
    ///
    /// # Arguments
    /// - `platform_version`: A reference to the platform version, used to determine which
    ///   update cost algorithm to apply.
    ///
    /// # Returns
    /// - `Ok(u64)`: The total update cost in credits for this update.
    /// - `Err(ProtocolError)`: If the platform version is unrecognized.
    ///
    /// # Version Behavior
    /// - Version 0: Always returns `0` (used before protocol version 9).
    /// - Version 1: For V0 transitions, uses registration cost logic for all items in the contract.
    ///              For V1 transitions, calculates cost based on updated_document_schemas.
    pub fn update_contract_cost(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .update_contract_cost
        {
            0 => Ok(0), // Before 2.0 it's just 0
            1 => {
                match self {
                    // V0 transitions embed the full contract, so we use the
                    // registration cost logic (charging for all schemas/tokens/keywords)
                    DataContractUpdateTransition::V0(v0) => {
                        let document_schemas = v0
                            .data_contract
                            .document_schemas()
                            .cloned()
                            .unwrap_or_default();
                        Ok(registration_cost_from_fields(
                            &document_schemas,
                            v0.data_contract.tokens(),
                            v0.data_contract.keywords().len(),
                            0, // no base fee for updates
                            platform_version,
                        ))
                    }
                    // V1 transitions explicitly specify updated_document_schemas
                    DataContractUpdateTransition::V1(v1) => Ok(update_contract_cost_from_fields(
                        &v1.updated_document_schemas,
                        platform_version,
                    )),
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractUpdateTransition::update_contract_cost".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
