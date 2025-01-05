use dpp::platform_value::Identifier;
use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::prelude::ConsensusValidationResult;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionV0};

impl TokenBaseTransitionAction {
    /// from base transition with contract lookup
    pub fn try_from_base_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenBaseTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        match value {
            TokenBaseTransition::V0(v0) => Ok(
                TokenBaseTransitionActionV0::try_from_base_transition_with_contract_lookup(
                    drive,
                    owner_id,
                    v0,
                    approximate_without_state_for_costs,
                    transaction,
                    drive_operations,
                    get_data_contract,
                    platform_version,
                )?
                .map(|v0| v0.into()),
            ),
        }
    }

    /// from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenBaseTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        match value {
            TokenBaseTransition::V0(v0) => Ok(
            TokenBaseTransitionActionV0::try_from_borrowed_base_transition_with_contract_lookup(
                drive,
                owner_id,
                v0,
                approximate_without_state_for_costs,
                transaction,
                drive_operations,
                get_data_contract,
                platform_version,
            )?.map(|v0| v0.into())
            ),
        }
    }
}
