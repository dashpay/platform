use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::prelude::DataContract;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

impl DataContract {
    /// Validates the provided groups to ensure they meet the requirements for data contracts.
    ///
    /// # Parameters
    /// - `groups`: A reference to a `BTreeMap` of group contract positions (`GroupContractPosition`)
    ///   mapped to their corresponding `Group` objects. These represent the groups associated with
    ///   the data contract.
    /// - `platform_version`: A reference to the [`PlatformVersion`](crate::version::PlatformVersion)
    ///   object specifying the version of the platform and determining which validation method to use.
    ///
    /// # Returns
    /// - `Ok(SimpleConsensusValidationResult)` if all the groups pass validation:
    ///   - Group contract positions must be contiguous, i.e., no gaps between positions.
    ///   - Each group must meet its individual validation criteria.
    /// - `Err(ProtocolError)` if:
    ///   - An unknown or unsupported platform version is provided.
    ///   - Validation of any group fails.
    ///
    /// # Behavior
    /// - Delegates the actual validation logic to the appropriate versioned implementation
    ///   (`validate_groups_v0`) based on the provided platform version.
    /// - If an unknown platform version is encountered, a `ProtocolError::UnknownVersionMismatch`
    ///   is returned.
    ///
    /// # Errors
    /// - Returns a `ProtocolError::UnknownVersionMismatch` if the platform version is not recognized.
    /// - Returns validation errors for:
    ///   - Non-contiguous group contract positions (`NonContiguousContractGroupPositionsError`).
    ///   - Invalid individual group configurations (e.g., power-related errors or exceeding member limits).
    pub fn validate_groups(
        groups: &BTreeMap<GroupContractPosition, Group>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_groups
        {
            0 => Self::validate_groups_v0(groups, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_groups".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
