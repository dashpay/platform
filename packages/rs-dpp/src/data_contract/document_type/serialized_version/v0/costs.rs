//! The stored form of what a document type charges and the trade it allows: token costs
//! ([`TokenCosts`]), action fees ([`DocumentActionFees`]), [`Transferable`] and [`TradeMode`].

use crate::data_contract::document_type::action_fees::v0::DocumentActionFeesV0;
use crate::data_contract::document_type::action_fees::{
    ActionFeePricing, DocumentActionFee, DocumentActionFees,
};
use crate::data_contract::document_type::token_costs::v0::TokenCostsV0;
use crate::data_contract::document_type::token_costs::TokenCosts;
use crate::document::transfer::Transferable;
use crate::nft::TradeMode;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_amount_on_contract_token::{
    DocumentActionTokenCost, DocumentActionTokenEffect,
};
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;

/// [`TokenCosts`], whose only generation is [`TokenCostsV0`].
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredTokenCosts {
    create: Option<StoredTokenCost>,
    replace: Option<StoredTokenCost>,
    delete: Option<StoredTokenCost>,
    transfer: Option<StoredTokenCost>,
    update_price: Option<StoredTokenCost>,
    purchase: Option<StoredTokenCost>,
}

/// [`DocumentActionTokenCost`].
#[derive(Debug, Clone, PartialEq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredTokenCost {
    contract_id: Option<Identifier>,
    token_contract_position: u16,
    token_amount: u64,
    effect: StoredTokenEffect,
    gas_fees_paid_by: GasFeesPaidBy,
    optional: bool,
}

/// [`DocumentActionTokenEffect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredTokenEffect {
    TransferTokenToContractOwner,
    BurnToken,
}

/// [`DocumentActionFees`], whose only generation is [`DocumentActionFeesV0`].
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredActionFees {
    pricing: ActionFeePricing,
    create: Option<StoredActionFee>,
    replace: Option<StoredActionFee>,
    delete: Option<StoredActionFee>,
    transfer: Option<StoredActionFee>,
    update_price: Option<StoredActionFee>,
    purchase: Option<StoredActionFee>,
}

/// [`DocumentActionFee`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) struct StoredActionFee {
    owner: u64,
    moderators: u64,
}

/// [`Transferable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredTransferable {
    Never,
    Always,
}

/// [`TradeMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub(super) enum StoredTradeMode {
    None,
    DirectPurchase,
}

impl From<&TokenCosts> for StoredTokenCosts {
    fn from(token_costs: &TokenCosts) -> Self {
        let TokenCosts::V0(TokenCostsV0 {
            create,
            replace,
            delete,
            transfer,
            update_price,
            purchase,
        }) = token_costs;
        let cost =
            |cost: &Option<DocumentActionTokenCost>| cost.as_ref().map(StoredTokenCost::from);
        StoredTokenCosts {
            create: cost(create),
            replace: cost(replace),
            delete: cost(delete),
            transfer: cost(transfer),
            update_price: cost(update_price),
            purchase: cost(purchase),
        }
    }
}

impl From<StoredTokenCosts> for TokenCosts {
    fn from(stored: StoredTokenCosts) -> Self {
        let StoredTokenCosts {
            create,
            replace,
            delete,
            transfer,
            update_price,
            purchase,
        } = stored;
        let cost = |cost: Option<StoredTokenCost>| cost.map(DocumentActionTokenCost::from);
        TokenCosts::V0(TokenCostsV0 {
            create: cost(create),
            replace: cost(replace),
            delete: cost(delete),
            transfer: cost(transfer),
            update_price: cost(update_price),
            purchase: cost(purchase),
        })
    }
}

impl From<&DocumentActionTokenCost> for StoredTokenCost {
    fn from(cost: &DocumentActionTokenCost) -> Self {
        let DocumentActionTokenCost {
            contract_id,
            token_contract_position,
            token_amount,
            effect,
            gas_fees_paid_by,
            optional,
        } = cost;
        StoredTokenCost {
            contract_id: *contract_id,
            token_contract_position: *token_contract_position,
            token_amount: *token_amount,
            effect: match effect {
                DocumentActionTokenEffect::TransferTokenToContractOwner => {
                    StoredTokenEffect::TransferTokenToContractOwner
                }
                DocumentActionTokenEffect::BurnToken => StoredTokenEffect::BurnToken,
            },
            gas_fees_paid_by: *gas_fees_paid_by,
            optional: *optional,
        }
    }
}

impl From<StoredTokenCost> for DocumentActionTokenCost {
    fn from(stored: StoredTokenCost) -> Self {
        let StoredTokenCost {
            contract_id,
            token_contract_position,
            token_amount,
            effect,
            gas_fees_paid_by,
            optional,
        } = stored;
        DocumentActionTokenCost {
            contract_id,
            token_contract_position,
            token_amount,
            effect: match effect {
                StoredTokenEffect::TransferTokenToContractOwner => {
                    DocumentActionTokenEffect::TransferTokenToContractOwner
                }
                StoredTokenEffect::BurnToken => DocumentActionTokenEffect::BurnToken,
            },
            gas_fees_paid_by,
            optional,
        }
    }
}

impl From<&DocumentActionFees> for StoredActionFees {
    fn from(fees: &DocumentActionFees) -> Self {
        let DocumentActionFees::V0(DocumentActionFeesV0 {
            pricing,
            create,
            replace,
            delete,
            transfer,
            update_price,
            purchase,
        }) = fees;
        let fee = |fee: &Option<DocumentActionFee>| {
            fee.as_ref().map(|fee| StoredActionFee {
                owner: fee.owner,
                moderators: fee.moderators,
            })
        };
        StoredActionFees {
            pricing: *pricing,
            create: fee(create),
            replace: fee(replace),
            delete: fee(delete),
            transfer: fee(transfer),
            update_price: fee(update_price),
            purchase: fee(purchase),
        }
    }
}

impl From<StoredActionFees> for DocumentActionFees {
    fn from(stored: StoredActionFees) -> Self {
        let StoredActionFees {
            pricing,
            create,
            replace,
            delete,
            transfer,
            update_price,
            purchase,
        } = stored;
        let fee = |fee: Option<StoredActionFee>| {
            fee.map(|fee| DocumentActionFee {
                owner: fee.owner,
                moderators: fee.moderators,
            })
        };
        DocumentActionFees::V0(DocumentActionFeesV0 {
            pricing,
            create: fee(create),
            replace: fee(replace),
            delete: fee(delete),
            transfer: fee(transfer),
            update_price: fee(update_price),
            purchase: fee(purchase),
        })
    }
}

impl From<Transferable> for StoredTransferable {
    fn from(transferable: Transferable) -> Self {
        match transferable {
            Transferable::Never => StoredTransferable::Never,
            Transferable::Always => StoredTransferable::Always,
        }
    }
}

impl From<StoredTransferable> for Transferable {
    fn from(stored: StoredTransferable) -> Self {
        match stored {
            StoredTransferable::Never => Transferable::Never,
            StoredTransferable::Always => Transferable::Always,
        }
    }
}

impl From<TradeMode> for StoredTradeMode {
    fn from(trade_mode: TradeMode) -> Self {
        match trade_mode {
            TradeMode::None => StoredTradeMode::None,
            TradeMode::DirectPurchase => StoredTradeMode::DirectPurchase,
        }
    }
}

impl From<StoredTradeMode> for TradeMode {
    fn from(stored: StoredTradeMode) -> Self {
        match stored {
            StoredTradeMode::None => TradeMode::None,
            StoredTradeMode::DirectPurchase => TradeMode::DirectPurchase,
        }
    }
}
