pub mod accessors;
mod property;
pub use property::*;
pub mod class_methods;
mod index;
pub mod methods;
pub use index::*;
mod index_level;
pub use index_level::IndexLevel;
pub use index_level::IndexLevelTypeInfo;
pub use index_level::IndexType;

#[cfg(feature = "random-documents")]
pub mod random_document;
pub mod restricted_creation;
pub mod schema;

mod token_costs;
pub mod v0;
pub mod v1;
#[cfg(feature = "validation")]
pub(crate) mod validator;

use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
use crate::document::Document;
use crate::fee::Credits;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;

pub const DEFAULT_HASH_SIZE: usize = 32;
pub const DEFAULT_FLOAT_SIZE: usize = 8;
pub const EMPTY_TREE_STORAGE_SIZE: usize = 33;
pub const MAX_INDEX_SIZE: usize = 255;
pub const STORAGE_FLAGS_SIZE: usize = 2;

pub(crate) mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";

    pub const CAN_BE_DELETED: &str = "canBeDeleted";
    pub const TRANSFERABLE: &str = "transferable";
    pub const TRADE_MODE: &str = "tradeMode";

    pub const CREATION_RESTRICTION_MODE: &str = "creationRestrictionMode";
    pub const SECURITY_LEVEL_REQUIREMENT: &str = "signatureSecurityLevelRequirement";
    pub const REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityEncryptionBoundedKey";
    pub const REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityDecryptionBoundedKey";
    pub const INDICES: &str = "indices";
    pub const NULL_SEARCHABLE: &str = "nullSearchable";
    pub const PROPERTIES: &str = "properties";
    pub const POSITION: &str = "position";
    pub const REQUIRED: &str = "required";
    pub const TRANSIENT: &str = "transient";
    pub const TYPE: &str = "type";
    pub const REF: &str = "$ref";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
    pub const TRANSFERRED_AT: &str = "$transferredAt";
    pub const MINIMUM: &str = "minimum";
    pub const ENUM: &str = "enum";
    pub const MAXIMUM: &str = "maximum";
    pub const MIN_ITEMS: &str = "minItems";
    pub const MAX_ITEMS: &str = "maxItems";
    pub const MIN_LENGTH: &str = "minLength";
    pub const MAX_LENGTH: &str = "maxLength";
    pub const BYTE_ARRAY: &str = "byteArray";
    pub const CONTENT_MEDIA_TYPE: &str = "contentMediaType";
    pub const ENCRYPTION_KEY_REQUIREMENTS: &str = "encryptionKeyReqs";
    pub const DECRYPTION_KEY_REQUIREMENTS: &str = "decryptionKeyReqs";
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DocumentTypeRef<'a> {
    V0(&'a DocumentTypeV0),
    V1(&'a DocumentTypeV1),
}

#[derive(Debug)]
pub enum DocumentTypeMutRef<'a> {
    V0(&'a mut DocumentTypeV0),
    V1(&'a mut DocumentTypeV1),
}

#[derive(Debug, Clone, PartialEq, From)]
pub enum DocumentType {
    V0(DocumentTypeV0),
    V1(DocumentTypeV1),
}

impl DocumentType {
    pub const fn as_ref(&self) -> DocumentTypeRef {
        match self {
            DocumentType::V0(v0) => DocumentTypeRef::V0(v0),
            DocumentType::V1(v1) => DocumentTypeRef::V1(v1),
        }
    }

    pub fn as_mut_ref(&mut self) -> DocumentTypeMutRef {
        match self {
            DocumentType::V0(v0) => DocumentTypeMutRef::V0(v0),
            DocumentType::V1(v1) => DocumentTypeMutRef::V1(v1),
        }
    }

    pub fn prefunded_voting_balances_for_document(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(String, Credits)>, ProtocolError> {
        match self {
            DocumentType::V0(v0) => {
                v0.prefunded_voting_balance_for_document(document, platform_version)
            }
            DocumentType::V1(v1) => {
                v1.prefunded_voting_balance_for_document(document, platform_version)
            }
        }
    }
}

impl DocumentTypeRef<'_> {
    pub fn to_owned_document_type(&self) -> DocumentType {
        match self {
            DocumentTypeRef::V0(v0) => DocumentType::V0((*v0).to_owned()),
            DocumentTypeRef::V1(v1) => DocumentType::V1((*v1).to_owned()),
        }
    }
}

impl DocumentTypeBasicMethods for DocumentType {}

impl DocumentTypeBasicMethods for DocumentTypeRef<'_> {}

impl DocumentTypeV0Methods for DocumentType {}

impl DocumentTypeV0Methods for DocumentTypeRef<'_> {}
