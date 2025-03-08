use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl DataContract {
    /// Compares two `DataContract` instances while ignoring time-related fields.
    ///
    /// This function checks for equality while excluding:
    /// - `created_at`
    /// - `updated_at`
    /// - `created_at_block_height`
    /// - `updated_at_block_height`
    /// - `created_at_epoch`
    /// - `updated_at_epoch`
    ///
    /// # Arguments
    /// - `other`: A reference to another `DataContract` to compare against.
    ///
    /// # Returns
    /// - `true` if all non-time fields match, otherwise `false`.
    pub fn equal_ignoring_time_fields(
        &self,
        other: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .equal_ignoring_time_fields
        {
            0 => Ok(self.equal_ignoring_time_fields_v0(other)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::equal_ignoring_time_fields".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
