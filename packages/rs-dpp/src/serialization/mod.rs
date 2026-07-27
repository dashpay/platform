#[cfg(feature = "json-conversion")]
pub mod json;
// Not gated behind `serde-conversion`: this fixed-size byte-array helper is
// needed by modules that derive serde unconditionally (e.g. the always-serde
// `block::extended_block_info::ExtendedBlockInfo`), so it must resolve in every
// feature configuration. It only depends on `serde` and `base64`, both
// non-optional dependencies.
#[cfg(feature = "serde-conversion")]
pub mod dashcore;
pub mod serde_bytes;
#[cfg(feature = "serde-conversion")]
pub mod serde_bytes_var;
pub(crate) mod serialization_traits;

pub use dpp_json_convertible_derive::json_safe_fields;
#[cfg(feature = "json-conversion")]
pub use dpp_json_convertible_derive::JsonConvertible;
#[cfg(feature = "value-conversion")]
pub use dpp_json_convertible_derive::ValueConvertible;
#[cfg(feature = "json-conversion")]
pub use json::safe_integer::{
    json_safe_i64, json_safe_option_i64, json_safe_option_u64, json_safe_u128,
    json_safe_u128_content, json_safe_u64,
};
#[cfg(feature = "json-conversion")]
pub use json::JsonSafeFields;
pub use serialization_traits::*;
