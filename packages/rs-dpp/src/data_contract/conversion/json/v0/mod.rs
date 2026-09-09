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
}
