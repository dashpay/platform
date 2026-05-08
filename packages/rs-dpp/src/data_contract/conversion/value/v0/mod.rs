use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

/// Version-aware platform_value conversion for `DataContract`.
///
/// **KEEP-AS-EXCEPTION** in the JSON/Value canonical-trait migration. Method
/// names use the `_versioned` suffix to disambiguate from canonical
/// `ValueConvertible::to_object` / `from_object` (which take no
/// `PlatformVersion`). See `data_contract/mod.rs` doc comment and the
/// unification plan §3.11 step 10 for the full rationale.
///
/// `DataContract` is a versioned enum routed through
/// `DataContractInSerializationFormat`. Both the platform version and the
/// `full_validation` flag are inputs to the conversion — they cannot be
/// expressed by the canonical traits' parameter-free signatures.
pub trait DataContractValueConversionMethodsV0 {
    /// Deserialize from a `platform_value::Value` at the given platform
    /// version, optionally running schema validation.
    fn from_value_versioned(
        raw_object: Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Returns Data Contract as a `platform_value::Value` at the given
    /// platform version.
    fn to_value_versioned(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Value, ProtocolError>;
    /// Consuming form of `to_value_versioned`.
    fn into_value_versioned(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Value, ProtocolError>;
}
