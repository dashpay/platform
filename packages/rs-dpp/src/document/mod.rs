pub use fields::{IDENTIFIER_FIELDS, property_names};

mod document;

mod accessors;
pub mod action_type;
mod document_patch;
pub mod errors;
pub mod extended_document;
pub mod generate_document_id;
mod serde_serialize;
mod v0;
#[cfg(feature = "json-object")]
mod json_conversion;
mod fields;

pub use v0::*;

pub use document::Document;
pub use extended_document::property_names as extended_document_property_names;
pub use extended_document::ExtendedDocument;
pub use extended_document::IDENTIFIER_FIELDS as EXTENDED_DOCUMENT_IDENTIFIER_FIELDS;

/// the initial revision of newly created document
pub const INITIAL_REVISION: u64 = 1;
