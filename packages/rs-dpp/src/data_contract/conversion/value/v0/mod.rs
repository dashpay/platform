use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

/// Validating `platform_value` deserialization for `DataContract`.
///
/// **KEEP-AS-EXCEPTION** in the JSON/Value canonical-trait migration —
/// canonical `ValueConvertible::from_object` does NOT run schema validation
/// (per the no-validation-by-default policy in `conversion/serde/mod.rs`).
/// This trait provides the opt-in validating path used by SDK boundaries,
/// fixture loaders, and validation tests.
///
/// The non-validating path lives on canonical `ValueConvertible` /
/// `platform_value::from_value::<DataContract>(...)` — use that when the
/// input has already been validated upstream (e.g., loading from storage).
///
/// For *serialization*, use canonical `ValueConvertible::to_object` /
/// `platform_value::to_value(&data_contract)` directly. There is no
/// validation dimension to writing.
///
/// See `data_contract/mod.rs` doc comment and the unification plan §3.11
/// step 10 for the full rationale.
pub trait DataContractValueConversionMethodsV0 {
    /// Deserialize from a `platform_value::Value` and run full schema
    /// validation. Use this on trust boundaries (SDK ingest, fixture
    /// loaders). For internal storage reads where validation already ran
    /// upstream, use canonical
    /// `platform_value::from_value::<DataContract>(...)` instead.
    fn from_value_validated(
        raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
