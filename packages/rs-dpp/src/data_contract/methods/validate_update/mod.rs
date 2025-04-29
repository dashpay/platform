use crate::block::block_info::BlockInfo;
use crate::data_contract::DataContract;
use platform_version::version::PlatformVersion;

mod v0;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
pub use v0::*;

impl DataContractUpdateValidationMethodsV0 for DataContract {
    fn validate_update(
        &self,
        data_contract: &DataContract,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .validate_update
        {
            0 => self.validate_update_v0(data_contract, block_info, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::validate_update".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
