#[cfg(feature = "data-contract-cbor-conversion")]
mod cbor;
#[cfg(feature = "data-contract-json-conversion")]
mod json;
#[cfg(feature = "data-contract-value-conversion")]
mod value;

// TODO: We need from_* / from_*_value / to_* / to_*_value methods for all types: cbor, json, platform_value (value?)
