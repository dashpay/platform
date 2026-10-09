use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::state_v0::DocumentBaseTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_shielded_pool_common::validate_token_not_paused;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::token::{IdentityTokenAccountFrozenError, TokenNotTransferableError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use dpp::identifier::Identifier;
use dpp::tokens::calculate_token_id;
use dpp::tokens::contract_info::v0::TokenContractInfoV0Accessors;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::error::drive::DriveError;
use drive::error::Error as DriveErrorWrapper;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
    DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentBaseTransitionActionStateValidationV2 {
    #[allow(clippy::too_many_arguments)]
    fn validate_state_v2(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        transition_type: &str,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentBaseTransitionActionStateValidationV2 for DocumentBaseTransitionAction {
    /// Version 2 (protocol version 14): transparent payments retain payer freeze and balance
    /// checks, refuse paying the contract owner in the contract's own non-transferable token,
    /// then enforce pause and the actual issuer's frozen-recipient policy for token movements. Elided owner self-payments retain only the payer checks; pool payments
    /// retain their separate shielded validation.
    fn validate_state_v2(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        transition_type: &str,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // Pool payments have their own pause, balance and proof validation after the document
        // action is valid; they do not spend the signer's transparent balance.
        if self.shielded_token_payment().is_some() {
            return Ok(SimpleConsensusValidationResult::new());
        }
        let validation_result = self.validate_state_v0(
            platform,
            owner_id,
            block_info,
            transition_type,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }
        let Some((token_id, effect, _)) = self.token_cost() else {
            return Ok(validation_result);
        };
        let recipient_id = self.data_contract_fetch_info_ref().contract.owner_id();
        // Owner self-payments emit no transfer. Preserve their payer checks without adding
        // movement restrictions or reads for an operation that will not happen.
        if effect == DocumentActionTokenEffect::TransferTokenToContractOwner
            && owner_id == recipient_id
        {
            return Ok(validation_result);
        }

        // A non-transferable token pays only by burning. Registration refuses any other cost in
        // one and the flag is fixed at creation; this holds the rule at payment time too for the
        // contract's own tokens, whose configuration is already in hand, so nothing is read.
        if effect == DocumentActionTokenEffect::TransferTokenToContractOwner {
            let contract = &self.data_contract_fetch_info_ref().contract;
            let own_non_transferable = contract.tokens().iter().any(|(position, configuration)| {
                !configuration.is_transferable()
                    && calculate_token_id(contract.id().as_bytes(), *position)
                        == token_id.to_buffer()
            });
            if own_non_transferable {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    TokenNotTransferableError::new(
                        token_id,
                        format!(
                            "a document {} token payment to the contract owner",
                            transition_type
                        ),
                    )
                    .into(),
                ));
            }
        }

        let validation_result = validate_token_not_paused(
            platform,
            token_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() || effect == DocumentActionTokenEffect::BurnToken {
            return Ok(validation_result);
        }

        let (recipient_info, fee) = platform.drive.fetch_identity_token_info_with_costs(
            token_id.to_buffer(),
            recipient_id.to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        if !recipient_info.is_some_and(|info| info.frozen()) {
            return Ok(validation_result);
        }

        // Only a frozen recipient needs the issuer's policy. The document contract can accept
        // another contract's token, so its own token settings cannot decide this restriction.
        let (token_contract_info, fee) = platform.drive.fetch_token_contract_info_with_costs(
            token_id.to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        let token_contract_info = token_contract_info.ok_or_else(|| {
            Error::Drive(DriveErrorWrapper::Drive(DriveError::CorruptedDriveState(
                format!("contract info is missing for document payment token {token_id}"),
            )))
        })?;
        let token_contract_id = token_contract_info.contract_id();
        let token_contract_position = token_contract_info.token_contract_position();
        if calculate_token_id(token_contract_id.as_bytes(), token_contract_position)
            != token_id.to_buffer()
        {
            return Err(Error::Drive(DriveErrorWrapper::Drive(
                DriveError::CorruptedDriveState(format!(
                    "contract info does not match document payment token {token_id}"
                )),
            )));
        }
        let token_contract = if token_contract_id == self.data_contract_id() {
            self.data_contract_fetch_info()
        } else {
            let (fee, contract) = platform.drive.get_contract_with_fetch_info_and_fee(
                token_contract_id.to_buffer(),
                Some(&block_info.epoch),
                false,
                transaction,
                platform_version,
            )?;
            let fee = fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "fee must exist when fetching a token's contract with an epoch",
            )))?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
            contract.ok_or_else(|| {
                Error::Drive(DriveErrorWrapper::Drive(DriveError::CorruptedDriveState(
                    format!("issuer contract {token_contract_id} is missing for document payment token {token_id}"),
                )))
            })?
        };
        let token_configuration = token_contract
            .contract
            .tokens()
            .get(&token_contract_position)
            .ok_or_else(|| {
                Error::Drive(DriveErrorWrapper::Drive(DriveError::CorruptedDriveState(
                    format!("issuer contract {token_contract_id} has no token at document payment position {token_contract_position}"),
                )))
            })?;
        if !token_configuration.is_allowed_transfer_to_frozen_balance() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityTokenAccountFrozenError::new(
                    token_id,
                    recipient_id,
                    format!("Document {} token payment", transition_type),
                )
                .into(),
            ));
        }
        Ok(validation_result)
    }
}
