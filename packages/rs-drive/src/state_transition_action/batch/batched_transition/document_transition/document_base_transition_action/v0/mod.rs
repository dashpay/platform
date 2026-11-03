/// transformer
pub mod transformer;

use crate::drive::contract::DataContractFetchInfo;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
use dpp::data_contract::document_type::action_fees::{ActionFeePricing, DocumentActionFee};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use dpp::ProtocolError;
use std::sync::Arc;

/// The fee a document type declares for an action, with what the transition agreed to pay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclaredDocumentActionFee {
    /// How the declared amounts become the amounts charged
    pub pricing: ActionFeePricing,
    /// The declared amounts
    pub fee: DocumentActionFee,
    /// The transition's action fee agreement, as it came on the wire. The batch's advanced
    /// structure validation judges it against the declaration, against the fee multiplier the
    /// batch transformer read, and, for a discounted moderators part, against the share of the
    /// contract's seated moderation charter the transformer read.
    pub agreement: Option<DocumentActionFeeAgreement>,
}

impl DeclaredDocumentActionFee {
    /// The amounts the action is charged before the fee multiplier: the declared ones, with
    /// the moderators part the agreement names when it asks for a discount on it (see
    /// [`DocumentActionFeeAgreement::discounts_moderators_of`]). Advanced structure validation
    /// refuses every discount but the one the contract's seated moderation charter gives, so
    /// only that one reaches execution, and the action is charged what it agreed to.
    pub fn agreed_fee(&self) -> DocumentActionFee {
        match self.agreement {
            Some(agreement) if agreement.discounts_moderators_of(self.pricing, self.fee) => {
                DocumentActionFee {
                    owner: self.fee.owner,
                    moderators: agreement.moderators(),
                }
            }
            _ => self.fee,
        }
    }
}

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
    /// Who the transition's token payment info asks to pay the gas
    pub gas_fees_paid_by: GasFeesPaidBy,
    /// Who the document type's token cost offers to pay the gas: `DocumentOwner` when the action
    /// has no token cost, since only a token payment can be sponsored
    pub contract_gas_fees_paid_by: GasFeesPaidBy,
    /// The fee the document type declares for this action, how it is priced and what the
    /// transition agreed to pay, `None` when it declares none (protocol version 14). Boxed:
    /// most actions declare none, and the action sits in the largest variant of the batched
    /// transition enum.
    pub declared_action_fee: Option<Box<DeclaredDocumentActionFee>>,
}

/// document base transition action accessors v0
pub trait DocumentBaseTransitionActionAccessorsV0 {
    /// The document ID
    fn id(&self) -> Identifier;

    /// The document type
    fn document_type(&self) -> Result<DocumentTypeRef<'_>, ProtocolError>;

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

    /// Who the transition's token payment info asks to pay the gas
    fn gas_fees_paid_by(&self) -> GasFeesPaidBy;

    /// Who the document type's token cost offers to pay the gas (`DocumentOwner` without a
    /// token cost)
    fn contract_gas_fees_paid_by(&self) -> GasFeesPaidBy;
    /// The fee the document type declares for this action, with what the transition agreed
    /// to pay
    fn declared_action_fee_with_agreement(&self) -> Option<DeclaredDocumentActionFee>;

    /// Whether the transition agrees to a discounted moderators part on a document type an
    /// elected contract moderates: the only place a discount may come from, the share of the
    /// contract's seated moderation charter. The batch transformer reads that share for every
    /// such contract, and advanced structure validation judges the agreement against it. An
    /// agreement to less anywhere else is a mismatch, judged without reading anything.
    fn agrees_to_a_moderators_discount(&self) -> bool;
}
