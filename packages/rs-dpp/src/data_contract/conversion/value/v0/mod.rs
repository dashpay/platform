use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

/// `platform_value` deserialization for `DataContract` with an explicit
/// validation flag.
///
/// `from_value(value, full_validation, pv)` is the single entry point: pass
/// `true` on trust boundaries (SDK ingest, fixture loaders, contract
/// registration) to run full schema validation, and `false` to reconstruct
/// already-trusted data (e.g. storage reads) without re-validating it.
///
/// For *serialization*, use canonical `ValueConvertible::to_object` /
/// `platform_value::to_value(&data_contract)` directly — there is no
/// validation dimension to writing.
pub trait DataContractValueConversionMethodsV0 {
    /// Deserialize from a `platform_value::Value`, running full schema
    /// validation when `full_validation` is `true`.
    fn from_value(
        raw_object: Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
