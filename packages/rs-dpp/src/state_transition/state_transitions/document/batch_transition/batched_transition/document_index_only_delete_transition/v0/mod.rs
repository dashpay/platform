mod from_document;
pub mod v0_methods;

use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use derive_more::Display;
use platform_value::Value;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub use super::super::document_base_transition::IDENTIFIER_FIELDS;

/// The indexOnly delete: carries the document's full property-value tuple
/// alongside the base.
///
/// An indexOnly document has no primary-storage row, so a delete cannot
/// fetch anything by id — the values in `data` (plus the signer as owner)
/// are what every index entry is recomputed from, the exact mirror of what
/// the create wrote. `$createdAt` rides in `data` under its system key
/// exactly when the document type requires it.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
// `Deserialize` is implemented manually below — same reason as
// `DocumentCreateTransitionV0`: two `#[serde(flatten)]` fields, one of
// which is a catchall map that would otherwise swallow the base's keys.
#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Display)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize),
    serde(rename_all = "camelCase")
)]
#[display("Base: {}, Data: {:?}", "base", "data")]
pub struct DocumentIndexOnlyDeleteTransitionV0 {
    /// Document Base Transition
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub base: DocumentBaseTransition,

    /// The property values of the indexOnly document being deleted.
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub data: BTreeMap<String, Value>,
}

// Manual `Deserialize`: peel the base's known keys off the flat object,
// reconstruct the base from them, and route everything left to `data`.
// See the WARNING on `DocumentCreateTransitionV0`'s impl — a new base
// field must be added to `BASE_FIELD_NAMES` here too.
#[cfg(feature = "serde-conversion")]
impl<'de> Deserialize<'de> for DocumentIndexOnlyDeleteTransitionV0 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        // Tag + every serde-renamed field of `DocumentBaseTransitionV0` /
        // `DocumentBaseTransitionV1`. Keep in sync with the base structs.
        const BASE_FIELD_NAMES: &[&str] = &[
            "$baseFormatVersion",
            "$id",
            "$identityContractNonce",
            "$type",
            "$dataContractId",
            "$tokenPaymentInfo",
        ];

        let mut map: BTreeMap<String, Value> = BTreeMap::deserialize(deserializer)?;

        let mut base_pairs: Vec<(Value, Value)> = Vec::with_capacity(BASE_FIELD_NAMES.len());
        for key in BASE_FIELD_NAMES {
            if let Some(value) = map.remove(*key) {
                base_pairs.push((Value::Text((*key).to_string()), value));
            }
        }
        let base = platform_value::from_value::<DocumentBaseTransition>(Value::Map(base_pairs))
            .map_err(D::Error::custom)?;

        Ok(DocumentIndexOnlyDeleteTransitionV0 { base, data: map })
    }
}
