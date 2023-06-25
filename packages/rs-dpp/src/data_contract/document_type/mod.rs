pub mod array_field;
pub mod document_field;
pub mod index;
pub mod v0;
pub mod index_level;

use crate::data_contract::document_type::v0::DocumentTypeV0;
use std::borrow::Cow;
pub use {
    array_field::v0::ArrayFieldTypeV0,
    document_field::v0::{DocumentFieldTypeV0, DocumentFieldV0},
    index::v0::{IndexV0, IndexPropertyV0},
};

pub(self) mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";
    pub const INDICES: &str = "indices";
    pub const PROPERTIES: &str = "properties";
    pub const REQUIRED: &str = "required";
    pub const TYPE: &str = "type";
    pub const REF: &str = "$ref";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
    pub const MIN_ITEMS: &str = "minItems";
    pub const MAX_ITEMS: &str = "maxItems";
    pub const MIN_LENGTH: &str = "minLength";
    pub const MAX_LENGTH: &str = "maxLength";
    pub const BYTE_ARRAY: &str = "byteArray";
    pub const CONTENT_MEDIA_TYPE: &str = "contentMediaType";
}

pub enum DocumentType<'a> {
    V0(Cow<'a, DocumentTypeV0>),
}
