mod v0;

use crate::block::block_info::BlockInfo;
use crate::data_contract::update_values::DataContractUpdateValues;
use crate::data_contract::DataContract;
use crate::validation::operations::ProtocolValidationOperation;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Merges a delta-based contract update onto this contract and returns
    /// the updated contract.
    ///
    /// This is the materialization step of a V1 data contract update
    /// transition: the caller fetched this contract from state, and the
    /// result is the contract the update would store. The result still has
    /// to pass [`validate_update`](crate::data_contract::validate_update::DataContractUpdateValidationMethodsV0::validate_update)
    /// against this contract, exactly like a full contract sent by a V0
    /// transition; this method only checks what the delta shape itself
    /// makes checkable (the submitter owns the contract, updated entries
    /// exist, new entries do not) and rebuilds the contract.
    ///
    /// # Arguments
    /// - `update_values`: the delta to apply.
    /// - `block_info`: the block the update executes in; it stamps the
    ///   `updatedAt` fields.
    /// - `full_validation`: whether to fully validate the rebuilt contract.
    /// - `validation_operations`: collects the validation work done for fees.
    /// - `platform_version`: the current platform version.
    ///
    /// # Returns
    /// - `Ok(ConsensusValidationResult<DataContract>)`: the updated contract,
    ///   or the consensus errors that reject the delta.
    /// - `Err(ProtocolError)`: the platform version is unknown, or rebuilding
    ///   the contract failed.
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
