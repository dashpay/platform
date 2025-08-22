/// transformer
pub mod transformer;

use crate::drive::contract::DataContractFetchInfo;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use dpp::ProtocolError;
use std::sync::Arc;

#[derive(Debug, Clone)]
/// document base transition action v0
pub struct DocumentBaseTransitionActionV0 {
    /// The document ID
    pub id: Identifier,
    /// The identity contract nonce, this is used to stop replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    pub document_type_name: String,
    /// A potential data contract
    pub data_contract: Arc<DataContractFetchInfo>,
    /// Token cost with the token_id coming first
    pub token_cost: Option<(Identifier, DocumentActionTokenEffect, TokenAmount)>,
    /// Who pays the gas fees
    pub gas_fees_paid_by: GasFeesPaidBy,
}

/// document base transition action accessors v0
pub trait DocumentBaseTransitionActionAccessorsV0 {
    /// The document ID
    fn id(&self) -> Identifier;

    /// The document type
    fn document_type(&self) -> Result<DocumentTypeRef, ProtocolError>;

    /// Is a field required on the document type?
    fn document_type_field_is_required(&self, field: &str) -> Result<bool, ProtocolError>;

    /// Name of document type found int the data contract associated with the `data_contract_id`
    fn document_type_name(&self) -> &String;
    /// document type name owned
    fn document_type_name_owned(self) -> String;
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    fn data_contract_id(&self) -> Identifier;

    /// A reference to the data contract fetch info that does not clone the Arc
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo>;
    /// Data contract
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo>;
    /// Identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;

    /// Token cost
    fn token_cost(&self) -> Option<(Identifier, DocumentActionTokenEffect, TokenAmount)>;
}
