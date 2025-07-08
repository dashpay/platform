mod v1;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContract {
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
    ) -> Result<u64, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .registration_cost
        {
            0 => Ok(0), // Before 2.0 it's just 0 (There was some validation cost)
            1 => Ok(self.registration_cost_v1(platform_version)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::registration_cost".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl DataContractInSerializationFormat {
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
    ) -> Result<u64, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .registration_cost
        {
            0 => Ok(0), // Before 2.0 it's just 0 (There was some validation cost)
            1 => Ok(self.registration_cost_v1(platform_version)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::registration_cost".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
