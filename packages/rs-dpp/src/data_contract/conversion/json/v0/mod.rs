use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

/// Version-aware JSON conversion for `DataContract`.
///
/// **KEEP-AS-EXCEPTION** in the JSON/Value canonical-trait migration. Method
/// names use the `_versioned` suffix to disambiguate from canonical
/// `JsonConvertible::to_json` / `from_json` (which take no `PlatformVersion`).
/// See `data_contract/mod.rs` doc comment and the unification plan §3.11
/// step 10 for the full rationale.
///
/// `DataContract` is a versioned enum routed through
/// `DataContractInSerializationFormat`. Both the platform version and the
/// `full_validation` flag are inputs to the conversion — they cannot be
/// expressed by the canonical traits' parameter-free signatures.
pub trait DataContractJsonConversionMethodsV0 {
    /// Deserialize from JSON at the given platform version, optionally
    /// running schema validation.
    fn from_json_versioned(
        json_value: JsonValue,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Returns Data Contract as a JSON Value at the given platform version.
    fn to_json_versioned(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError>;

    /// Returns Data Contract as a validating-JSON Value at the given
    /// platform version (used by JSON Schema validators that don't accept
    /// base64 string encodings of binary data).
    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError>;
}
