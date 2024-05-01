mod enrich_with_base_schema;

mod find_identifier_and_binary_paths;

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
