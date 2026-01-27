use std::collections::BTreeMap;

use crate::prelude::DataContract;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;

mod v0;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

impl DataContract {
    /// Validates schema $defs updates for compatibility with existing definitions.
    ///
    /// This method validates that:
    /// 1. Updated schema_defs reference existing definitions (can't update non-existent defs)
    /// 2. Updated schema_defs are compatible with the old definitions (schema compatibility)
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the data contract these schema defs belong to.
    /// - `old_schema_defs`: The existing schema $defs (if any) from the current contract.
    /// - `updated_schema_defs`: Schema definitions being updated (must exist in old_schema_defs).
    /// - `new_schema_defs`: New schema definitions being added.
    /// - `platform_version`: A reference to the [`PlatformVersion`] specifying which
    ///   validation method version to use.
    ///
    /// # Returns
    /// - `Ok(SimpleConsensusValidationResult)` containing validation errors if any:
    ///   - `IncompatibleDataContractSchemaError` if trying to update non-existent defs
    ///   - `IncompatibleDataContractSchemaError` if updated defs are incompatible
    /// - `Err(ProtocolError)` if an unknown or unsupported platform version is provided.
    pub fn validate_schema_defs_update(
        contract_id: Identifier,
        old_schema_defs: Option<&BTreeMap<String, Value>>,
        updated_schema_defs: &BTreeMap<String, Value>,
        new_schema_defs: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_schema_defs_update
        {
            0 => Self::validate_schema_defs_update_v0(
                contract_id,
                old_schema_defs,
                updated_schema_defs,
                new_schema_defs,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_schema_defs_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
