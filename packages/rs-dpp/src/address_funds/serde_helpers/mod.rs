//! Field-level serde helpers for address-based transition fields.
//!
//! These helpers reshape the JSON / wasm Object output of `BTreeMap<PlatformAddress, _>`
//! and `Option<(PlatformAddress, _)>` fields from an opaque map-of-tuples into a
//! self-describing array (or single object) of `{ address, nonce?, amount? }` entries.
//!
//! Only serde JSON / `platform_value` output is affected — the bincode `Encode` /
//! `Decode` derives on the parent transitions are independent of serde and remain
//! unchanged, so consensus binary format and `PlatformSignable` sighash are
//! intentionally untouched. Same safety argument as the custom-serde change applied
//! to `AddressFundsFeeStrategyStep`.
//!
//! Each helper exposes `pub fn serialize` and `pub fn deserialize` so it can be
//! attached to a struct field via `#[serde(with = "...")]`.
//!
//! Module gating lives on the parent re-export in `address_funds/mod.rs`
//! (`#[cfg(feature = "json-conversion")]`), so this file does not need its
//! own inner `#![cfg(...)]` attribute.

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::serialization::json_safe_fields;
use serde::{Deserialize, Serialize};

pub mod address_input_map;
pub mod address_output_map_optional_amount;
pub mod address_output_map_required_amount;
pub mod address_output_singular;

/// Wire shape for an output address entry: `{ address, amount }` with required amount.
/// Shared between the `address_output_map_required_amount` and `address_output_singular`
/// helpers so the JSON shape stays consistent across plural / singular fields.
///
/// `#[json_safe_fields]` auto-applies `json_safe_u64` to the `amount` field so values
/// above `Number.MAX_SAFE_INTEGER` (2^53 − 1) are stringified in human-readable JSON.
#[json_safe_fields]
#[derive(Serialize, Deserialize)]
pub(crate) struct AddressOutputEntry {
    pub(crate) address: PlatformAddress,
    pub(crate) amount: Credits,
}
