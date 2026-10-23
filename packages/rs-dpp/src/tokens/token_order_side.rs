use crate::document::Document;
use crate::prelude::IdentityNonce;
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
use crate::serialization::JsonConvertible;
#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
use crate::serialization::ValueConvertible;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Identifier;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::fmt;

/// The side of a token marketplace order (protocol version 15): a listing (`Ask`), which holds
/// the tokens it offers for sale in escrow, or an offer (`Bid`), which holds the credits it offers
/// in escrow. Each side is its own document type of the token market contract.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Encode,
    Decode,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum TokenOrderSide {
    /// A listing: tokens for sale at a price.
    #[default]
    Ask = 0,
    /// An offer: credits to buy tokens at a price.
    Bid = 1,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for TokenOrderSide {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl ValueConvertible for TokenOrderSide {}

impl TokenOrderSide {
    /// The token market contract document type that holds the orders of this side.
    pub fn document_type_name(&self) -> &'static str {
        match self {
            TokenOrderSide::Ask => "ask",
            TokenOrderSide::Bid => "bid",
        }
    }
}

impl fmt::Display for TokenOrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenOrderSide::Ask => write!(f, "ask"),
            TokenOrderSide::Bid => write!(f, "bid"),
        }
    }
}

impl TokenOrderSide {
    /// The id of the order of this side that `maker` places for `token_id` with the identity
    /// contract nonce `nonce`: known before the transition is broadcast, and unique, since the
    /// nonce is used once. It is the id of the order's document in the token market contract and
    /// the key of its escrow entry. The token id takes the contract id's place, as it does in
    /// the ids of token history documents.
    pub fn order_id(
        &self,
        token_id: Identifier,
        maker: Identifier,
        nonce: IdentityNonce,
    ) -> Identifier {
        Document::generate_document_id_v0(
            &token_id,
            &maker,
            self.document_type_name(),
            nonce.to_be_bytes().as_slice(),
        )
    }

    /// The number of bytes `order_id` hashes (twice, with SHA-256): the token id and the maker's
    /// id (32 bytes each), the document type name and the nonce.
    pub fn order_id_preimage_size(&self) -> usize {
        2 * 32 + self.document_type_name().len() + size_of::<IdentityNonce>()
    }
}
