use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;

impl TokenConfiguration {
    /// Validates that all group positions referenced in the token configuration exist in the provided groups map.
    ///
    /// # Parameters
    /// - `groups`: A reference to a `BTreeMap` containing group positions as keys and their associated `Group` objects as values.
    ///   These represent the groups defined in the data contract.
    /// - `platform_version`: A reference to the `PlatformVersion` object specifying the version of the function to call.
    ///
    /// # Returns
    /// - `Ok(SimpleConsensusValidationResult)`: If the validation is successful, returns a result containing a validation result object,
    ///   which will be empty if no errors are found.
    /// - `Err(ProtocolError)`: If an unknown or unsupported platform version is specified, an error indicating a version mismatch is returned.
    ///
    /// # Errors
    /// - If a group position referenced in the token configuration does not exist in the provided `groups` map, the method will invoke the
    ///   version-specific validation logic, which will include any corresponding validation errors.
    ///
    /// # Versioning
    /// - This function dispatches to version-specific validation logic based on the `platform_version`.
    /// - Currently supports `validate_token_config_groups_exist_v0` for version `0`.
    pub fn validate_token_config_groups_exist(
        &self,
        groups: &BTreeMap<GroupContractPosition, Group>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .data_contract
            .validate_token_config_groups_exist
        {
            0 => Ok(self.validate_token_config_groups_exist_v0(groups)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_token_config_groups_exist".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
