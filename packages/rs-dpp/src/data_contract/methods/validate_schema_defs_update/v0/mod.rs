use std::collections::BTreeMap;

use crate::consensus::basic::data_contract::IncompatibleDataContractSchemaError;
use crate::data_contract::document_type::schema::validate_schema_compatibility;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde_json::json;

impl DataContract {
    /// Validates schema $defs updates for compatibility with existing definitions.
    ///
    /// This method validates that:
    /// 1. Updated schema_defs reference existing definitions
    /// 2. Updated schema_defs are compatible with the old definitions
    ///
    /// # Arguments
    /// - `contract_id`: The identifier of the data contract.
    /// - `old_schema_defs`: The existing schema $defs (if any).
    /// - `updated_schema_defs`: The schema $defs being updated.
    /// - `new_schema_defs`: The new schema $defs being added.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok(SimpleConsensusValidationResult)` containing validation errors if any.
    /// - `Err(ProtocolError)` if a protocol error occurs.
    #[inline(always)]
    pub(super) fn validate_schema_defs_update_v0(
        contract_id: Identifier,
        old_schema_defs: Option<&BTreeMap<String, Value>>,
        updated_schema_defs: &BTreeMap<String, Value>,
        new_schema_defs: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // If no updates, nothing to validate
        if updated_schema_defs.is_empty() && new_schema_defs.is_empty() {
            return Ok(SimpleConsensusValidationResult::new());
        }

        // Validate that updated schema_defs reference existing ones
        if let Some(old_defs_map) = old_schema_defs {
            for schema_def_name in updated_schema_defs.keys() {
                if !old_defs_map.contains_key(schema_def_name) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        IncompatibleDataContractSchemaError::new(
                            contract_id,
                            "add".to_string(),
                            format!("/$defs/{}", schema_def_name),
                        )
                        .into(),
                    ));
                }
            }
        } else if !updated_schema_defs.is_empty() {
            // Can't update schema defs if none exist
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IncompatibleDataContractSchemaError::new(
                    contract_id,
                    "update".to_string(),
                    "/$defs".to_string(),
                )
                .into(),
            ));
        }

        // If we have updates, validate compatibility
        if !updated_schema_defs.is_empty() {
            if let Some(old_defs_map) = old_schema_defs {
                // Build merged new defs
                let mut merged_defs = old_defs_map.clone();
                for (name, def) in updated_schema_defs {
                    merged_defs.insert(name.clone(), def.clone());
                }
                for (name, def) in new_schema_defs {
                    merged_defs.insert(name.clone(), def.clone());
                }

                // Validate compatibility
                if old_defs_map != &merged_defs {
                    let old_defs_json = Value::from(old_defs_map)
                        .try_into_validating_json()
                        .map_err(ProtocolError::ValueError)?;

                    let new_defs_json = Value::from(&merged_defs)
                        .try_into_validating_json()
                        .map_err(ProtocolError::ValueError)?;

                    let old_defs_schema = json!({
                        "$defs": old_defs_json
                    });

                    let new_defs_schema = json!({
                        "$defs": new_defs_json
                    });

                    let compatibility_validation_result = validate_schema_compatibility(
                        &old_defs_schema,
                        &new_defs_schema,
                        platform_version,
                    )?;

                    if !compatibility_validation_result.is_valid() {
                        let errors = compatibility_validation_result
                            .errors
                            .into_iter()
                            .map(|operation| {
                                IncompatibleDataContractSchemaError::new(
                                    contract_id,
                                    operation.name,
                                    operation.path,
                                )
                                .into()
                            })
                            .collect();

                        return Ok(SimpleConsensusValidationResult::new_with_errors(errors));
                    }
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
