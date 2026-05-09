use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

/// Validating JSON deserialization for `DataContract`.
///
/// **KEEP-AS-EXCEPTION** in the JSON/Value canonical-trait migration —
/// canonical `JsonConvertible::from_json` does NOT run schema validation
/// (per the no-validation-by-default policy in `conversion/serde/mod.rs`).
/// This trait provides the opt-in validating path used by SDK boundaries,
/// JSON-fixture loaders, and validation tests.
///
/// The non-validating path lives on canonical `JsonConvertible` /
/// `serde_json::from_value::<DataContract>(...)` — use that when the input
/// has already been validated upstream (e.g., loading from storage).
///
/// For *serialization*, use canonical `JsonConvertible::to_json` /
/// `serde_json::to_value(&data_contract)` directly. There is no validation
/// dimension to writing.
///
/// See `data_contract/mod.rs` doc comment and the unification plan §3.11
/// step 10 for the full rationale.
pub trait DataContractJsonConversionMethodsV0 {
    /// Deserialize from JSON and run full schema validation. Use this on
    /// trust boundaries (SDK ingest, gRPC handlers, fixture loaders).
    /// For internal storage reads where validation already ran upstream,
    /// use canonical `serde_json::from_value::<DataContract>(...)` instead.
    fn from_json_validated(
        json_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Returns Data Contract as a validating-JSON Value at the given
    /// platform version (used by JSON Schema validators that don't accept
    /// base64 string encodings of binary data).
    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError>;
}
