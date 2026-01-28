mod v0;

use crate::block::block_info::BlockInfo;
use crate::data_contract::update_values::DataContractUpdateValues;
use crate::data_contract::DataContract;
use crate::validation::operations::ProtocolValidationOperation;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Applies update values to this contract to produce a new contract.
    ///
    /// This method takes the current contract and applies the changes specified in the
    /// update values to produce a new updated contract. It merges document schemas,
    /// groups, tokens, and keywords from the update into the existing contract.
    ///
    /// This method dispatches to a version-specific implementation based on the
    /// platform version configuration. If the version is unrecognized, it returns a version mismatch error.
    ///
    /// # Arguments
    /// - `update_values`: The update values containing the changes to apply.
    /// - `block_info`: Block information containing timestamp, epoch, and height for the update.
    /// - `full_validation`: Whether to perform full validation on the resulting contract.
    /// - `validation_operations`: A vector to collect validation operations.
    /// - `platform_version`: The current platform version.
    ///
    /// # Returns
    /// - `Ok(ConsensusValidationResult<DataContract>)`: The validation result containing the new contract if successful.
    /// - `Err(ProtocolError)`: If the platform version is unrecognized or if a protocol error occurs.
    ///
    /// # Version Behavior
    /// - Version 0: Applies updates using the standard merge logic for document schemas,
    ///   groups, tokens, keywords, and description.
    pub fn apply_update(
        &self,
        update_values: DataContractUpdateValues<'_>,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<DataContract>, ProtocolError> {
        match platform_version.dpp.contract_versions.methods.apply_update {
            0 => self.apply_update_v0(
                update_values,
                block_info,
                full_validation,
                validation_operations,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::apply_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
