mod v0;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::update_values::DataContractUpdateValues;
use crate::data_contract::DataContract;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::feature_initial_protocol_versions::DATA_CONTRACT_UPDATE_V1_INITIAL_PROTOCOL_VERSION;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Merges a delta contract update onto this contract, the stored one, and returns the
    /// contract the update asks for, in the format state transitions carry.
    ///
    /// This is the one definition of what a delta means. Block execution runs it on the
    /// contract it reads from state and then holds the result to every rule of an update that
    /// carries the whole contract; the verification of a delta's execution proof runs it on
    /// the contract the client held before the update and compares the result with the proved
    /// contract. Both dispatch on `contract_versions.methods.apply_update`, so they always
    /// merge alike.
    ///
    /// Only what the shape of a delta makes checkable is checked here: the submitter owns the
    /// contract, an entry to replace or remove exists, and an entry to add does not. The
    /// merged contract keeps this contract's time fields; the caller stamps the update's.
    ///
    /// # Returns
    /// - The merged contract, or the consensus error that says why the delta does not fit this
    ///   contract.
    /// - `Err(ProtocolError)` when `platform_version` predates delta contract updates (the
    ///   slot is `None`) or selects a generation this build does not know.
    pub fn apply_update(
        &self,
        update_values: DataContractUpdateValues<'_>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<DataContractInSerializationFormat>, ProtocolError> {
        match platform_version.dpp.contract_versions.methods.apply_update {
            None => Err(ProtocolError::UnknownVersionError(format!(
                "DataContract::apply_update is not active at protocol version {}, delta contract updates activate with protocol version {}",
                platform_version.protocol_version, DATA_CONTRACT_UPDATE_V1_INITIAL_PROTOCOL_VERSION
            ))),
            Some(0) => Ok(self.apply_update_v0(update_values)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::apply_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
