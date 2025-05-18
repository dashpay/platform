use dpp::block::epoch::Epoch;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Getters;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::{GroupOperationType, TokenOperationType};
use crate::util::batch::DriveOperation::{GroupOperation, IdentityOperation, TokenOperation};

impl DriveHighLevelBatchOperationConverter for TokenMintTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .token_mint_transition
        {
            0 => {
                let data_contract_id = self.base().data_contract_id();

                let identity_contract_nonce = self.base().identity_contract_nonce();

                let mut ops = vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityContractNonce {
                        identity_id: owner_id.into_buffer(),
                        contract_id: data_contract_id.into_buffer(),
                        nonce: identity_contract_nonce,
                    },
                )];

                if let Some(GroupStateTransitionResolvedInfo {
                    group_contract_position,
                    action_id,
                    action_is_proposer,
                    signer_power,
                    ..
                }) = self.base().store_in_group()
                {
                    let event = TokenEvent::Mint(
                        self.mint_amount(),
                        self.identity_balance_holder_id(),
                        self.public_note().cloned(),
                    );

                    let initialize_with_insert_action_info = if *action_is_proposer {
                        Some(GroupAction::V0(GroupActionV0 {
                            contract_id: self.base().data_contract_id(),
                            proposer_id: owner_id,
                            token_contract_position: self.base().token_position(),
                            event: GroupActionEvent::TokenEvent(event),
                        }))
                    } else {
                        None
                    };

                    ops.push(GroupOperation(GroupOperationType::AddGroupAction {
                        contract_id: data_contract_id,
                        group_contract_position: *group_contract_position,
                        initialize_with_insert_action_info,
                        action_id: *action_id,
                        signer_identity_id: owner_id,
                        signer_power: *signer_power,
                        closes_group_action: self.base().perform_action(),
                    }));
                }

                if self.base().perform_action() {
                    ops.push(TokenOperation(TokenOperationType::TokenMint {
                        token_id: self.token_id(),
                        identity_balance_holder_id: self.identity_balance_holder_id(),
                        mint_amount: self.mint_amount(),
                        allow_first_mint: false,
                        allow_saturation: false,
                    }));

                    let token_configuration = self.base().token_configuration()?;
                    if token_configuration.keeps_history().keeps_minting_history() {
                        ops.push(TokenOperation(TokenOperationType::TokenHistory {
                            token_id: self.token_id(),
                            owner_id,
                            nonce: identity_contract_nonce,
                            event: TokenEvent::Mint(
                                self.mint_amount(),
                                self.identity_balance_holder_id(),
                                self.public_note_owned(),
                            ),
                        }));
                    }
                }

                Ok(ops)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "TokenMintTransitionAction::into_high_level_document_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
