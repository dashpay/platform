use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

/// `platform_value` deserialization for `DataContract` with an explicit
/// validation flag.
///
/// `from_value(value, full_validation, pv)` is the single entry point: pass
/// `true` on trust boundaries (SDK ingest, fixture loaders, contract
/// registration) to run full schema validation, and `false` to reconstruct
/// already-trusted data (e.g. storage reads) without re-validating it.
///
/// For *serialization*, `to_value(pv)` threads an explicit platform version.
/// The serde path (`ValueConvertible::to_object` /
/// `platform_value::to_value(&data_contract)`) instead reads the
/// process-global current platform version, which another thread may change
/// concurrently (e.g. parallel tests building platforms at older protocol
/// versions) — prefer `to_value(pv)` wherever a `PlatformVersion` is in hand.
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

    /// Serialize to a `platform_value::Value` at an explicit platform
    /// version, independent of the process-global current version.
    ///
    /// Provided by default for any implementor convertible to
    /// `DataContractInSerializationFormat` (`DataContract`, `DataContractV0`,
    /// `DataContractV1`); other implementors of this trait need not provide
    /// it, keeping the added method source-compatible.
    fn to_value(&self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError>
    where
        Self: Sized,
        for<'a> DataContractInSerializationFormat:
            TryFromPlatformVersioned<&'a Self, Error = ProtocolError>,
    {
        let format: DataContractInSerializationFormat =
            self.try_into_platform_versioned(platform_version)?;
        platform_value::to_value(&format).map_err(ProtocolError::ValueError)
    }
}
