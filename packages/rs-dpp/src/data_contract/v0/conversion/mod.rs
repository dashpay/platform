#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "json-object")]
mod json;
#[cfg(feature = "platform-value")]
mod platform_value;

// TODO: We need from_* / from_*_value / to_* / to_*_value methods for all types: cbor, json, platform_value (value?)
