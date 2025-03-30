pub mod v0_accessors;

use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use crate::ProtocolError;
use bincode_derive::{Decode, Encode};
use derive_more::Display;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::{Identifier, Value};
#[cfg(any(
    feature = "state-transition-serde-conversion",
    all(
        feature = "document-serde-conversion",
        feature = "data-contract-serde-conversion"
    ),
))]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(
    any(
        feature = "state-transition-serde-conversion",
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
    ),
    derive(Serialize, Deserialize)
)]
#[display(
    "Contract ID: {:?}, Token Position: {:?}, Min Cost: {:?}, Max Cost: {:?}, Gas Fees Paid By: {}",
    payment_token_contract_id,
    token_contract_position,
    minimum_token_cost,
    maximum_token_cost,
    gas_fees_paid_by
)]
pub struct TokenPaymentInfoV0 {
    /// By default, we use a token in the same contract, this field must be set if the document
    /// requires payment using another contracts token.
    pub payment_token_contract_id: Option<Identifier>,
    /// If we are expecting to pay with a token in a contract, which token are we expecting
    /// to pay with?
    /// We have this set so contract owners can't switch out to more valuable token.
    /// For example if my Data contract
    pub token_contract_position: TokenContractPosition,
    /// Minimum token cost, this most often should not be set
    pub minimum_token_cost: Option<TokenAmount>,
    /// Maximum token cost, this most often should be set
    /// If:
    /// - a client does not have this set
    /// - and the data contract allows the price of NFTs to be changed by the data contract's owner or allowed party.
    ///   Then:
    /// - The user could see the cost changed on them
    pub maximum_token_cost: Option<TokenAmount>,
    /// Who pays the gas fees, this needs to match what the contract allows
    pub gas_fees_paid_by: GasFeesPaidBy,
}

impl TokenPaymentInfoAccessorsV0 for TokenPaymentInfoV0 {
    // Getters
    fn payment_token_contract_id(&self) -> Option<Identifier> {
        self.payment_token_contract_id
    }

    fn payment_token_contract_id_ref(&self) -> &Option<Identifier> {
        &self.payment_token_contract_id
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        self.token_contract_position
    }

    fn minimum_token_cost(&self) -> Option<TokenAmount> {
        self.minimum_token_cost
    }

    fn maximum_token_cost(&self) -> Option<TokenAmount> {
        self.maximum_token_cost
    }

    // Setters
    fn set_payment_token_contract_id(&mut self, id: Option<Identifier>) {
        self.payment_token_contract_id = id;
    }

    fn set_token_contract_position(&mut self, position: TokenContractPosition) {
        self.token_contract_position = position;
    }

    fn set_minimum_token_cost(&mut self, cost: Option<TokenAmount>) {
        self.minimum_token_cost = cost;
    }

    fn set_maximum_token_cost(&mut self, cost: Option<TokenAmount>) {
        self.maximum_token_cost = cost;
    }

    fn gas_fees_paid_by(&self) -> GasFeesPaidBy {
        self.gas_fees_paid_by
    }

    fn set_gas_fees_paid_by(&mut self, payer: GasFeesPaidBy) {
        self.gas_fees_paid_by = payer;
    }
}

impl TryFrom<BTreeMap<String, Value>> for TokenPaymentInfoV0 {
    type Error = ProtocolError;

    fn try_from(mut map: BTreeMap<String, Value>) -> Result<Self, Self::Error> {
        Ok(TokenPaymentInfoV0 {
            payment_token_contract_id: map.remove_optional_identifier("paymentTokenContractId")?,

            token_contract_position: map
                .remove_optional_integer("tokenContractPosition")?
                .unwrap_or_default(),

            minimum_token_cost: map.remove_optional_integer("minimumTokenCost")?,

            maximum_token_cost: map.remove_optional_integer("maximumTokenCost")?,

            gas_fees_paid_by: map
                .remove_optional_string("gasFeesPaidBy")?
                .map(|v| match v.as_str() {
                    "DocumentOwner" => GasFeesPaidBy::DocumentOwner,
                    "ContractOwner" => GasFeesPaidBy::ContractOwner,
                    "PreferContractOwner" => GasFeesPaidBy::PreferContractOwner,
                    _ => GasFeesPaidBy::default(),
                })
                .unwrap_or_default(),
        })
    }
}
