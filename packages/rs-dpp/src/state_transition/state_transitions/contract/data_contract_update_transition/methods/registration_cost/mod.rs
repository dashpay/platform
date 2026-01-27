mod v1;

use crate::fee::Credits;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::state_transitions::contract::data_contract_create_transition::methods::registration_cost::registration_cost_from_fields;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransition {
    /// Returns the registration cost of the data contract update based on the current platform version.
    ///
    /// For V0 transitions, this calculates cost based on the embedded data contract's schemas, tokens, and keywords.
    /// For V1 transitions, this calculates the cost based on new items being added
    /// (new document schemas, new tokens, added keywords).
    ///
    /// # Arguments
    /// - `platform_version`: A reference to the platform version, used to determine which
    ///   registration cost algorithm to apply.
    ///
    /// # Returns
    /// - `Ok(u64)`: The total registration cost in credits for this update.
    /// - `Err(ProtocolError)`: If the platform version is unrecognized.
    ///
    /// # Version Behavior
    /// - Version 0: Always returns `0` (used before protocol version 9).
    /// - Version 1: Uses a detailed cost model for items being registered.
    ///              Note: For updates, there is no base contract fee (only new items are charged).
    pub fn registration_cost(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .registration_cost
        {
            0 => Ok(0), // Before 2.0 it's just 0
            1 => {
                // For updates, no base fee - only charge for new items
                let base_fee: Credits = 0;

                match self {
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
                            base_fee,
                            platform_version,
                        ))
                    }
                    DataContractUpdateTransition::V1(v1) => Ok(registration_cost_from_fields(
                        &v1.new_document_schemas,
                        &v1.new_tokens,
                        v1.add_keywords.len(),
                        base_fee,
                        platform_version,
                    )),
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractUpdateTransition::registration_cost".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
