use crate::data_contract::document_type::token_costs::accessors::{
    TokenCostGettersV0, TokenCostSettersV0,
};
use crate::data_contract::document_type::token_costs::v0::TokenCostsV0;
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use derive_more::From;

pub(crate) mod accessors;
pub mod v0;

/// The token costs for various document operations
#[derive(Debug, PartialEq, Clone, From)]
pub enum TokenCosts {
    /// Version 0 of token costs
    V0(TokenCostsV0),
}

/// Implementation of the `TokenCostGettersV0` trait for `TokenCosts` enum.
impl TokenCostGettersV0 for TokenCosts {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_creation_token_cost(),
        }
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_replacement_token_cost(),
        }
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_deletion_token_cost(),
        }
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_transfer_token_cost(),
        }
    }

    fn document_price_update_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_price_update_token_cost(),
        }
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_purchase_token_cost(),
        }
    }

    fn document_creation_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_creation_token_cost_ref(),
        }
    }

    fn document_replacement_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_replacement_token_cost_ref(),
        }
    }

    fn document_deletion_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_deletion_token_cost_ref(),
        }
    }

    fn document_transfer_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_transfer_token_cost_ref(),
        }
    }

    fn document_price_update_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_price_update_token_cost_ref(),
        }
    }

    fn document_purchase_token_cost_ref(&self) -> Option<&DocumentActionTokenCost> {
        match self {
            TokenCosts::V0(inner) => inner.document_purchase_token_cost_ref(),
        }
    }
}

impl TokenCostSettersV0 for TokenCosts {
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            TokenCosts::V0(inner) => inner.set_document_creation_token_cost(cost),
        }
    }

    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            TokenCosts::V0(inner) => inner.set_document_replacement_token_cost(cost),
        }
    }

    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            TokenCosts::V0(inner) => inner.set_document_deletion_token_cost(cost),
        }
    }

    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            TokenCosts::V0(inner) => inner.set_document_transfer_token_cost(cost),
        }
    }

    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            TokenCosts::V0(inner) => inner.set_document_price_update_token_cost(cost),
        }
    }

    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        match self {
            TokenCosts::V0(inner) => inner.set_document_purchase_token_cost(cost),
        }
    }
}
