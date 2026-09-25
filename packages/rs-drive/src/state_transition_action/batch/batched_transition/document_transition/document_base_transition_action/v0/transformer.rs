use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::action_fees::{DocumentActionFee, DocumentActionFees};
use dpp::data_contract::document_type::DocumentType;
use dpp::platform_value::Identifier;
use std::sync::Arc;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityHasNotAgreedToPayRequiredTokenAmountError, IdentityTryingToPayWithWrongTokenError, RequiredTokenPaymentInfoNotSetError};
use dpp::prelude::ConsensusValidationResult;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::state_transition::batch_transition::document_base_transition::v2::v2_methods::DocumentBaseTransitionV2Methods;
use dpp::tokens::calculate_token_id;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use dpp::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use dpp::tokens::token_payment_info::methods::v0::TokenPaymentInfoMethodsV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DeclaredDocumentActionFee, DocumentBaseTransitionActionV0};

impl DocumentBaseTransitionActionV0 {
    /// try from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        value: &DocumentBaseTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        get_token_cost: impl Fn(&DocumentType) -> Option<DocumentActionTokenCost>,
        get_action_fee: impl Fn(&DocumentActionFees) -> Option<DocumentActionFee>,
        action: &str,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        let data_contract_id = value.data_contract_id();
        let data_contract = get_data_contract(data_contract_id)?;
        let document_type = data_contract
            .contract
            .document_type_borrowed_for_name(value.document_type_name().as_str())?;
        // An optional token cost is waived by leaving the token payment info out: the action is
        // then paid for in credits by its signer, like one without a token cost, and offers no
        // gas sponsorship. Only a v3 meta-schema contract (protocol version 14) carries the flag.
        let document_action_token_cost = get_token_cost(document_type)
            .filter(|cost| !(cost.optional && value.token_payment_info_ref().is_none()));
        let token_cost = document_action_token_cost.map(
            |DocumentActionTokenCost {
                 contract_id,
                 token_contract_position,
                 token_amount,
                 effect,
                 ..
             }| {
                (
                    calculate_token_id(
                        contract_id.unwrap_or(data_contract_id).as_bytes(),
                        token_contract_position,
                    )
                    .into(),
                    effect,
                    token_amount,
                )
            },
        );
        if let Some(document_action_token_cost) = document_action_token_cost {
            let Some(token_payment_info) = value.token_payment_info_ref() else {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::RequiredTokenPaymentInfoNotSetError(
                        RequiredTokenPaymentInfoNotSetError::new(
                            token_cost.expect("expected token cost").0,
                            action.to_string(),
                        ),
                    )),
                ));
            };
            // Let's see that the user agreed to pay using the correct token
            if !token_payment_info.matches_token_contract(
                &document_action_token_cost.contract_id,
                document_action_token_cost.token_contract_position,
            ) {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::IdentityTryingToPayWithWrongTokenError(
                        IdentityTryingToPayWithWrongTokenError::new(
                            document_action_token_cost.contract_id,
                            document_action_token_cost.token_contract_position,
                            token_cost.expect("expected token cost").0,
                            token_payment_info.payment_token_contract_id(),
                            token_payment_info.token_contract_position(),
                            token_payment_info.token_id(data_contract_id),
                        ),
                    )),
                ));
            }
            // Let's see that the user agreed to pay the required amount
            if !token_payment_info
                .is_valid_for_required_cost(document_action_token_cost.token_amount)
            {
                return Ok(ConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::IdentityHasNotAgreedToPayRequiredTokenAmountError(
                            IdentityHasNotAgreedToPayRequiredTokenAmountError::new(
                                token_cost.expect("expected token cost").0,
                                document_action_token_cost.token_amount,
                                token_payment_info.minimum_token_cost(),
                                token_payment_info.maximum_token_cost(),
                                action.to_string(),
                            ),
                        ),
                    ),
                ));
            }
        }
        let gas_fees_paid_by = value
            .token_payment_info_ref()
            .as_ref()
            .map(|token_payment_info| token_payment_info.gas_fees_paid_by())
            .unwrap_or(GasFeesPaidBy::DocumentOwner);
        // Both sides of the gas question travel on the action; from protocol version 14 the
        // batch's advanced structure validation refuses a request the offer does not cover and
        // the execution event charges whoever `GasFeesPaidBy::resolve` names.
        let contract_gas_fees_paid_by = document_action_token_cost
            .map(|cost| cost.gas_fees_paid_by)
            .unwrap_or(GasFeesPaidBy::DocumentOwner);
        // The fee the document type declares for this action, with how it is priced. Only a v3
        // meta-schema contract (protocol version 14) carries one. What is charged is settled
        // where state is read, since the default pricing follows the epoch's fee multiplier.
        // What the transition agreed to pay travels beside it: the batch's advanced structure
        // validation judges the two once the fee multiplier is known. An agreement on an action
        // that declares no fee is dropped, since nothing is charged.
        let declared_action_fee = document_type.action_fees().and_then(|fees| {
            get_action_fee(fees).map(|fee| {
                Box::new(DeclaredDocumentActionFee {
                    pricing: fees.pricing(),
                    fee,
                    agreement: value.action_fee_agreement(),
                })
            })
        });
        Ok(DocumentBaseTransitionActionV0 {
            id: value.id(),
            identity_contract_nonce: value.identity_contract_nonce(),
            document_type_name: value.document_type_name().clone(),
            data_contract,
            token_cost,
            gas_fees_paid_by,
            contract_gas_fees_paid_by,
            declared_action_fee,
        }
        .into())
    }
}
