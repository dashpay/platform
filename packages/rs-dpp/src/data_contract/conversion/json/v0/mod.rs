use crate::version::PlatformVersion;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

/// JSON deserialization for `DataContract` with an explicit validation flag.
///
/// `from_json(value, full_validation, pv)` is the single entry point: pass
/// `true` on trust boundaries (SDK ingest, gRPC handlers, fixture loaders) to
/// run full schema validation, and `false` to reconstruct already-trusted data
/// (e.g. storage reads) without re-validating it.
///
/// `from_json_validated` is a convenience for the `true` case.
///
/// For *serialization*, use canonical `JsonConvertible::to_json` /
/// `serde_json::to_value(&data_contract)` directly — there is no validation
/// dimension to writing.
pub trait DataContractJsonConversionMethodsV0 {
    /// Deserialize from JSON, running full schema validation when
    /// `full_validation` is `true`.
    fn from_json(
        json_value: JsonValue,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;

    /// Convenience for `from_json(json_value, true, platform_version)` — full
    /// schema validation. Use on trust boundaries.
    fn from_json_validated(
        json_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        Self::from_json(json_value, true, platform_version)
    }

    /// Returns Data Contract as a validating-JSON Value at the given
    /// platform version (used by JSON Schema validators that don't accept
    /// base64 string encodings of binary data).
    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError>;
}
