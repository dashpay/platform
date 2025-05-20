use dpp::platform_value::Identifier;
use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::prelude::ConsensusValidationResult;
use dpp::ProtocolError;
use dpp::state_transition::state_transitions::document::batch_transition::batched_transition::token_base_transition::TokenBaseTransition;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionV0};

/// A type representing if we need to change the state transition public note to the original group public note
pub type ChangeToOriginalPublicNote = Option<Option<String>>;

impl TokenBaseTransitionAction {
    /// from base transition with contract lookup
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_base_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenBaseTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<(Self, ChangeToOriginalPublicNote)>, Error> {
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
                .map(|(v0, change_note)| (v0.into(), change_note)),
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<ConsensusValidationResult<(Self, ChangeToOriginalPublicNote)>, Error> {
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
            )?.map(|(v0, change_note)| (v0.into(), change_note)),
            ),
        }
    }
}
