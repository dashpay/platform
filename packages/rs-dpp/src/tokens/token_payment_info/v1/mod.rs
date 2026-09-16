pub mod v1_accessors;

use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::shielded::SerializedAction;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use crate::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::btreemap_extensions::{
    BTreeValueRemoveFromMapHelper, BTreeValueRemoveInnerValueFromMapHelper,
};
use platform_value::{Identifier, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// A spend bundle in the payment token's shielded pool that pays a document action's token
/// cost. The notes it spends leave the pool: `amount` of them go where the document type's
/// token cost effect sends them (the contract owner's balance, or out of the supply) and the
/// change returns to the pool as new notes. The note owner authorizes the spend; the batch
/// owner still signs the batch and pays its fee in credits.
#[derive(Debug, Clone, Encode, Decode, PartialEq, DecodeUntrusted)]
// Auto-injects `json_safe_u64` on `amount` and `serde_bytes` on the fixed byte arrays /
// `serde_bytes_var` on `proof` (base64 strings in JSON, raw bytes in Value).
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenShieldedPayment {
    /// The tokens the bundle pays: its value balance. Must equal the token cost of the document
    /// action it pays for.
    pub amount: TokenAmount,
    /// Orchard actions (spend-output pairs).
    pub actions: Vec<SerializedAction>,
    /// Sinsemilla root of the token pool's note commitment tree the bundle was built against.
    pub anchor: [u8; 32],
    /// Halo 2 proof bytes.
    pub proof: Vec<u8>,
    /// RedPallas binding signature.
    pub binding_signature: [u8; 64],
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenShieldedPayment {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenShieldedPayment {}

impl Default for TokenShieldedPayment {
    fn default() -> Self {
        Self {
            amount: 0,
            actions: vec![],
            anchor: [0u8; 32],
            proof: vec![],
            binding_signature: [0u8; 64],
        }
    }
}

impl fmt::Display for TokenShieldedPayment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Shielded Payment: amount {}, actions {}",
            self.amount,
            self.actions.len()
        )
    }
}

impl TryFrom<BTreeMap<String, Value>> for TokenShieldedPayment {
    type Error = ProtocolError;

    fn try_from(mut map: BTreeMap<String, Value>) -> Result<Self, Self::Error> {
        let actions = map
            .remove_inner_value_array::<Vec<Value>>("actions")?
            .into_iter()
            .map(|value| {
                let mut action = value.into_btree_string_map()?;
                Ok(SerializedAction {
                    nullifier: action.remove_hash256_bytes("nullifier")?,
                    rk: action.remove_hash256_bytes("rk")?,
                    cmx: action.remove_hash256_bytes("cmx")?,
                    encrypted_note: action.remove_bytes("encryptedNote")?,
                    cv_net: action.remove_hash256_bytes("cvNet")?,
                    spend_auth_sig: fixed_64(action.remove_bytes("spendAuthSig")?, "spendAuthSig")?,
                })
            })
            .collect::<Result<Vec<_>, ProtocolError>>()?;
        Ok(TokenShieldedPayment {
            amount: map.remove_integer("amount")?,
            actions,
            anchor: map.remove_hash256_bytes("anchor")?,
            proof: map.remove_bytes("proof")?,
            binding_signature: fixed_64(map.remove_bytes("bindingSignature")?, "bindingSignature")?,
        })
    }
}

fn fixed_64(bytes: Vec<u8>, field: &str) -> Result<[u8; 64], ProtocolError> {
    let len = bytes.len();
    bytes
        .try_into()
        .map_err(|_| ProtocolError::DecodingError(format!("{field} must be 64 bytes, got {len}")))
}

/// Format version 1 of the token payment info: the `V0` fields plus a shielded payment. The
/// document action's token cost is paid out of the token's shielded pool by `shielded_payment`
/// instead of the document owner's token balance; the identity signing the batch still pays
/// the credit fee and receives nothing.
#[derive(Debug, Clone, Encode, Decode, Default, PartialEq, DecodeUntrusted)]
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct TokenPaymentInfoV1 {
    /// By default, we use a token in the same contract, this field must be set if the document
    /// requires payment using another contracts token.
    pub payment_token_contract_id: Option<Identifier>,
    /// Which token (by position) on the contract the payment uses.
    pub token_contract_position: TokenContractPosition,
    /// Minimum token cost, this most often should not be set.
    pub minimum_token_cost: Option<TokenAmount>,
    /// Maximum token cost, this most often should be set.
    pub maximum_token_cost: Option<TokenAmount>,
    /// Who pays the gas fees, this needs to match what the contract allows.
    pub gas_fees_paid_by: GasFeesPaidBy,
    /// The spend bundle paying the token cost from the token's shielded pool. Boxed so a
    /// payment info without one stays small.
    pub shielded_payment: Box<TokenShieldedPayment>,
}

impl fmt::Display for TokenPaymentInfoV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Contract ID: {:?}, Token Position: {:?}, Min Cost: {:?}, Max Cost: {:?}, Gas Fees Paid By: {}, {}",
            self.payment_token_contract_id,
            self.token_contract_position,
            self.minimum_token_cost,
            self.maximum_token_cost,
            self.gas_fees_paid_by,
            self.shielded_payment
        )
    }
}

impl TokenPaymentInfoV1 {
    /// Builds a `V1` from a `V0` and the shielded payment that pays its cost.
    pub fn from_v0(v0: TokenPaymentInfoV0, shielded_payment: TokenShieldedPayment) -> Self {
        Self {
            payment_token_contract_id: v0.payment_token_contract_id,
            token_contract_position: v0.token_contract_position,
            minimum_token_cost: v0.minimum_token_cost,
            maximum_token_cost: v0.maximum_token_cost,
            gas_fees_paid_by: v0.gas_fees_paid_by,
            shielded_payment: Box::new(shielded_payment),
        }
    }

    /// Drops the shielded payment, leaving the `V0` fields.
    pub fn into_v0(self) -> TokenPaymentInfoV0 {
        TokenPaymentInfoV0 {
            payment_token_contract_id: self.payment_token_contract_id,
            token_contract_position: self.token_contract_position,
            minimum_token_cost: self.minimum_token_cost,
            maximum_token_cost: self.maximum_token_cost,
            gas_fees_paid_by: self.gas_fees_paid_by,
        }
    }

    /// The spend bundle paying the token cost from the token's shielded pool.
    pub fn shielded_payment(&self) -> &TokenShieldedPayment {
        &self.shielded_payment
    }
}

impl TokenPaymentInfoAccessorsV0 for TokenPaymentInfoV1 {
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

impl TryFrom<BTreeMap<String, Value>> for TokenPaymentInfoV1 {
    type Error = ProtocolError;

    fn try_from(mut map: BTreeMap<String, Value>) -> Result<Self, Self::Error> {
        let shielded_map: BTreeMap<String, Value> = map
            .remove_map_as_btree_map_keep_values_as_platform_value::<String, Value>(
                "shieldedPayment",
            )?;
        let shielded_payment: TokenShieldedPayment = shielded_map.try_into()?;
        let v0: TokenPaymentInfoV0 = map.try_into()?;
        Ok(TokenPaymentInfoV1::from_v0(v0, shielded_payment))
    }
}
