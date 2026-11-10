use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::contract_user_moderation::state::v0::{
    fetch_moderated_contract, transform_document_fields_change_v0,
    ContractUserModerationStateTransitionStateValidationV0,
};
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::contract_user_moderation) trait ContractUserModerationStateTransitionStateValidationV1
{
    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ContractUserModerationStateTransitionStateValidationV1 for ContractUserModerationTransition {
    /// Version 1 (protocol version 15) transforms a document field change so that the change
    /// leaves the document's storage with whoever holds it. The read of the stored document
    /// also returns who its storage flags name, and the action carries that identity for Drive
    /// to write the change in its name.
    ///
    /// Version 0 writes the change in the name of the document's owner. From protocol version
    /// 15 a document a gas sponsor paid for is held by the contract owner, and a write naming
    /// another owner hands such a document over: the contract owner would be refunded and the
    /// moderator charged for the whole document, which its owner could then delete and collect.
    ///
    /// Every other action transforms through version 0, unchanged.
    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let ContractUserModerationAction::ChangeDocumentFields {
            document_type_name,
            document_id,
            fields,
            ..
        } = self.action()
        else {
            return self.transform_into_action_v0(
                platform,
                block_info,
                execution_context,
                tx,
                platform_version,
            );
        };

        // The contract is read, and its absence refused, as version 0 does for every action.
        let contract_id = self.data_contract_id();
        let Some(contract_fetch_info) = fetch_moderated_contract(
            contract_id,
            platform,
            block_info,
            execution_context,
            tx,
            platform_version,
        )?
        else {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_contract_user_moderation_transition(
                        self,
                    ),
                ),
                vec![DataContractNotPresentError::new(contract_id).into()],
            ));
        };

        transform_document_fields_change_v0(
            self,
            platform,
            block_info,
            &contract_fetch_info,
            document_type_name,
            *document_id,
            fields,
            true,
            execution_context,
            tx,
            platform_version,
        )
    }
}
