use dpp::consensus::state::token::TokenNotTransferableError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV1Getters};
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::TokenContractPosition;
use dpp::identifier::Identifier;
use dpp::tokens::calculate_token_id;

/// The refusal of `document_type`'s token costs in the non-transferable token at
/// `token_position` of the contract `contract_id`, naming each `tokenCost` action that charges
/// it. Such a cost always pays the contract owner, since another contract's token cannot be
/// burned.
pub(in crate::execution) fn external_non_transferable_token_cost_error(
    document_type: &DocumentType,
    contract_id: Identifier,
    token_position: TokenContractPosition,
) -> ConsensusError {
    let actions: Vec<&str> = [
        ("create", document_type.document_creation_token_cost()),
        ("replace", document_type.document_replacement_token_cost()),
        ("delete", document_type.document_deletion_token_cost()),
        ("transfer", document_type.document_transfer_token_cost()),
        (
            "update_price",
            document_type.document_update_price_token_cost(),
        ),
        ("purchase", document_type.document_purchase_token_cost()),
    ]
    .into_iter()
    .filter(|(_, cost)| {
        cost.is_some_and(|cost| {
            cost.contract_id == Some(contract_id) && cost.token_contract_position == token_position
        })
    })
    .map(|(action, _)| action)
    .collect();
    TokenNotTransferableError::new(
        calculate_token_id(contract_id.as_bytes(), token_position).into(),
        format!(
            "document type {}'s {} token cost (it pays the contract owner)",
            document_type.name(),
            actions.join(", ")
        ),
    )
    .into()
}
