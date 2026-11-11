use platform_value::Value;

pub mod allowed_top_level_properties;

mod enrich_with_base_schema;

mod expand_property_type_shorthands;

mod find_identifier_and_binary_paths;

#[cfg(feature = "validation-schema")]
pub mod validation_schema;

#[cfg(feature = "validation")]
mod recursive_schema_validator;

#[cfg(feature = "validation")]
pub use recursive_schema_validator::*;

#[cfg(feature = "validation")]
mod validate_max_depth;

#[cfg(feature = "validation")]
mod validate_schema_compatibility;
#[cfg(feature = "validation")]
pub use validate_schema_compatibility::*;

#[cfg(feature = "validation")]
pub use validate_max_depth::*;

/// The value of the last `key` entry of `map`, the entry every other reader of a property
/// schema takes (the parser's string map and the JSON the meta-schema and the validator read
/// both keep the last of a repeated key). `expand_property_type_shorthands` v0, which
/// protocol versions 14 and 15 select, reads through this, so its result is consensus
/// behavior: a reader that needs another rule must use a new helper.
pub(crate) fn last_value<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter()
        .rev()
        .find(|(entry_key, _)| entry_key.as_text() == Some(key))
        .map(|(_, value)| value)
}
