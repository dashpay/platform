mod v0;

use crate::block::block_info::BlockInfo;
use crate::data_contract::update_values::DataContractUpdateValues;
use crate::data_contract::DataContract;
use crate::validation::operations::ProtocolValidationOperation;
use crate::validation::ConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::feature_initial_protocol_versions::DATA_CONTRACT_UPDATE_V1_INITIAL_PROTOCOL_VERSION;
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
    /// - `Err(ProtocolError)`: the platform version predates delta-based
    ///   updates (the slot is `None`), is unknown, or rebuilding the contract
    ///   failed.
    pub fn apply_update(
        &self,
        update_values: DataContractUpdateValues<'_>,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<DataContract>, ProtocolError> {
        match platform_version.dpp.contract_versions.methods.apply_update {
            None => Err(ProtocolError::UnknownVersionError(format!(
                "DataContract::apply_update is not active at protocol version {}, delta-based contract updates arrive with protocol version {}",
                platform_version.protocol_version, DATA_CONTRACT_UPDATE_V1_INITIAL_PROTOCOL_VERSION
            ))),
            Some(0) => self.apply_update_v0(
                update_values,
                block_info,
                full_validation,
                validation_operations,
                platform_version,
            ),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::apply_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::update_values::{DataContractUpdateValues, DescriptionUpdate};
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use std::collections::BTreeMap;

    #[test]
    fn apply_update_is_not_active_before_protocol_version_15() {
        let platform_version_14 = PlatformVersion::get(14).expect("protocol version 14");
        let contract = get_data_contract_fixture(None, 0, platform_version_14.protocol_version)
            .data_contract_owned();
        let empty = BTreeMap::new();
        let update_values = DataContractUpdateValues {
            owner_id: contract.owner_id(),
            version: contract.version() + 1,
            config: None,
            updated_schema_defs: &empty,
            new_schema_defs: &empty,
            updated_document_schemas: &empty,
            new_document_schemas: &empty,
            new_groups: &BTreeMap::new(),
            new_tokens: &BTreeMap::new(),
            add_keywords: &[],
            remove_keywords: &[],
            description: &DescriptionUpdate::Keep,
        };

        let result = contract.apply_update(
            update_values,
            &BlockInfo::default(),
            false,
            &mut vec![],
            platform_version_14,
        );

        assert_matches!(result, Err(ProtocolError::UnknownVersionError(message)) if message.contains("not active"));
    }
}
