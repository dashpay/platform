use dpp::block::epoch::Epoch;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::identifier::Identifier;
use dpp::tokens::token_event::TokenEvent;
use platform_version::version::PlatformVersion;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::document::DriveHighLevelDocumentOperationConverter;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation::{IdentityOperation, TokenOperation};

impl DriveHighLevelDocumentOperationConverter for TokenMintTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
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

                ops.push(TokenOperation(TokenOperationType::TokenMint {
                    token_id: self.token_id(),
                    identity_balance_holder_id: self.identity_balance_holder_id(),
                    mint_amount: self.mint_amount(),
                    allow_first_mint: false,
                }));

                let token_configuration = self.base().token_configuration()?;
                if token_configuration.keeps_history() {
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
