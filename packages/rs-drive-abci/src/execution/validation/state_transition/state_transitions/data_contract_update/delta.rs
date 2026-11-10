//! A delta contract update (version 1 of the transition, from protocol version 15), merged
//! onto the stored contract.
//!
//! A delta carries only what an update changes, so nothing can be said of the contract it
//! asks for until the stored contract is read. Block execution (state validation) and
//! check_tx (the transformation into an action) both start here: the stored contract is read,
//! the delta is merged onto it, and what comes back is the update embedding the whole merged
//! contract, which the caller then validates with the generations such an update runs. A
//! delta is therefore held to every rule of an update that embeds its contract, by the code
//! that holds that update to them.

use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV1,
};
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::state_transitions::data_contract_update::basic_structure::v2::DataContractUpdateStateTransitionBasicStructureValidationV2;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

/// What became of a delta once the stored contract was read.
pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update)
enum MergedDelta {
    /// The delta is refused: the result holds the nonce bump and the errors.
    Refused(ConsensusValidationResult<StateTransitionAction>),
    /// The delta merged onto the stored contract.
    Merged {
        /// The update embedding the whole merged contract, under the delta's nonce and fee
        /// increase.
        full_contract_update: DataContractUpdateTransition,
        /// The registration cost of what the delta registers, which its action pays in place
        /// of the whole contract's.
        registration_cost: Credits,
    },
}

fn refused(transition: &DataContractUpdateTransition, errors: Vec<ConsensusError>) -> MergedDelta {
    MergedDelta::Refused(ConsensusValidationResult::new_with_data_and_errors(
        StateTransitionAction::BumpIdentityDataContractNonceAction(
            BumpIdentityDataContractNonceAction::from_borrowed_data_contract_update_transition(
                transition,
            ),
        ),
        errors,
    ))
}

/// Reads the contract `delta` names and merges the delta onto it.
///
/// The read is billed whether or not the contract exists, as the read of an update that embeds
/// its contract is. A contract that does not exist, a delta that does not fit the stored
/// contract (another owner, an entry to replace or remove that it does not have, an entry to
/// add that it has), and a merged contract that breaks a structure rule are refused with a
/// nonce bump, never with an execution error. The structure rules are those basic structure
/// generation 2 holds an update that embeds its contract to, run here on the update embedding
/// the merged contract, as basic structure cannot read the stored contract.
///
/// `transition` is the delta as a transition, which the nonce bump is built from.
#[allow(clippy::too_many_arguments)]
pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) fn merge_delta_onto_stored_contract<
    C: CoreRPCLike,
>(
    transition: &DataContractUpdateTransition,
    delta: &DataContractUpdateTransitionV1,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    validation_mode: ValidationMode,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<MergedDelta, Error> {
    let (fee, contract_fetch_info) = platform.drive.get_contract_with_fetch_info_and_fee(
        delta.data_contract_id.to_buffer(),
        Some(&block_info.epoch),
        validation_mode.can_alter_cache(),
        tx,
        platform_version,
    )?;
    let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
        "the read of the contract a delta contract update names must return its fee",
    )))?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

    let Some(contract_fetch_info) = contract_fetch_info else {
        return Ok(refused(
            transition,
            vec![
                BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                    delta.data_contract_id,
                ))
                .into(),
            ],
        ));
    };

    let merged = delta.to_full_contract_update(&contract_fetch_info.contract, platform_version)?;
    if !merged.is_valid() {
        return Ok(refused(transition, merged.errors));
    }
    let full_contract_update: DataContractUpdateTransition = merged.into_data()?.into();

    let structure_result = full_contract_update
        .validate_basic_structure_v2(platform.config.network, platform_version)?;
    if !structure_result.is_valid() {
        return Ok(refused(transition, structure_result.errors));
    }

    Ok(MergedDelta::Merged {
        full_contract_update,
        registration_cost: delta.registration_cost(platform_version)?,
    })
}

/// The result of validating the update embedding a merged delta, as the delta's: its action
/// pays `registration_cost`, the cost of what the delta registers, in place of the whole
/// contract's. A nonce bump is the delta's as it is, since the update carries the delta's
/// owner, contract, nonce and fee increase.
pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) fn as_delta_result(
    mut result: ConsensusValidationResult<StateTransitionAction>,
    registration_cost: Credits,
) -> ConsensusValidationResult<StateTransitionAction> {
    result.data = result.data.map(|action| match action {
        StateTransitionAction::DataContractUpdateAction(action) => {
            StateTransitionAction::DataContractUpdateAction(
                action.into_delta_action(registration_cost),
            )
        }
        action => action,
    });
    result
}
