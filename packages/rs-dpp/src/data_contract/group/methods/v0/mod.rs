use crate::data_contract::GroupContractPosition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub trait GroupMethodsV0 {
    fn validate(
        &self,
        group_contract_position: Option<GroupContractPosition>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
