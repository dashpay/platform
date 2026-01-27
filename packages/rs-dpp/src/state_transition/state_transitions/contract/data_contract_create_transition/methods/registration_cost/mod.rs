mod v1;

pub(in crate::state_transition::state_transitions::contract) use v1::registration_cost_from_fields;

use crate::fee::Credits;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransition {
    /// Returns the registration cost of the data contract based on the current platform version
    /// and the number of associated search keywords.
    ///
    /// This method dispatches to a version-specific implementation based on the
    /// platform version configuration. If the version is unrecognized, it returns a version mismatch error.
    ///
    /// # Arguments
    /// - `platform_version`: A reference to the platform version, used to determine which
    ///   registration cost algorithm to apply.
    ///
    /// # Returns
    /// - `Ok(u64)`: The total registration cost in credits for this contract.
    /// - `Err(ProtocolError)`: If the platform version is unrecognized or if the fee computation overflows.
    ///
    /// # Version Behavior
    /// - Version 0: Always returns `0` (used before protocol version 9, ie before 2.0, where registration cost was not charged).
    /// - Version 1: Uses a detailed cost model based on document types, indexes, tokens, and keyword count.
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
            0 => Ok(0), // Before 2.0 it's just 0 (There was some validation cost)
            1 => {
                let base_fee = platform_version
                    .fee_version
                    .data_contract_registration
                    .base_contract_registration_fee;

                match self {
                    DataContractCreateTransition::V0(v0) => {
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
                    DataContractCreateTransition::V1(v1) => Ok(registration_cost_from_fields(
                        &v1.document_schemas,
                        &v1.tokens,
                        v1.keywords.len(),
                        base_fee,
                        platform_version,
                    )),
                }
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractCreateTransition::registration_cost".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
