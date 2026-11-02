use crate::data_contract::document_type::action_fees::{ActionFeePricing, DocumentActionFee};

/// The action fees of a document type, version 0
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct DocumentActionFeesV0 {
    /// How the declared amounts become the amounts charged
    pub pricing: ActionFeePricing,

    /// Fee of creating a document
    pub create: Option<DocumentActionFee>,

    /// Fee of replacing a document
    pub replace: Option<DocumentActionFee>,

    /// Fee of deleting a document
    pub delete: Option<DocumentActionFee>,

    /// Fee of transferring a document
    pub transfer: Option<DocumentActionFee>,

    /// Fee of updating the price of a document
    pub update_price: Option<DocumentActionFee>,

    /// Fee of purchasing a document
    pub purchase: Option<DocumentActionFee>,
}
