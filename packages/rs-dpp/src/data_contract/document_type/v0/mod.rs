use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;

#[cfg(feature = "validation")]
pub use validator::StatelessJsonSchemaLazyValidator;

use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::document::transfer::Transferable;
use crate::identity::identity_public_key::SecurityLevel;
use crate::nft::TradeMode;
use platform_value::{Identifier, Value};

pub mod accessors;
#[cfg(feature = "random-documents")]
pub mod random_document;
#[cfg(feature = "random-document-types")]
pub mod random_document_type;
#[cfg(feature = "validation")]
pub mod validator;
pub const DEFAULT_HASH_SIZE: usize = 32;
pub const DEFAULT_FLOAT_SIZE: usize = 8;
pub const EMPTY_TREE_STORAGE_SIZE: usize = 33;
pub const MAX_INDEX_SIZE: usize = 255;
pub const STORAGE_FLAGS_SIZE: usize = 2;

#[derive(Debug, PartialEq, Clone)]
#[ferment_macro::export]
pub struct DocumentTypeV0 {
    pub name: String,
    pub schema: Value,
    pub indices: BTreeMap<String, Index>,
    pub index_structure: IndexLevel,
    /// Flattened properties flatten all objects for quick lookups for indexes
    /// Document field should not contain sub objects.
    pub flattened_properties: IndexMap<String, DocumentProperty>,
    /// Document field can contain sub objects.
    pub properties: IndexMap<String, DocumentProperty>,
    pub identifier_paths: BTreeSet<String>,
    pub binary_paths: BTreeSet<String>,
    /// The required fields on the document type
    pub required_fields: BTreeSet<String>,
    /// The transient fields on the document type
    pub transient_fields: BTreeSet<String>,
    /// Should documents keep history?
    pub documents_keep_history: bool,
    /// Are documents mutable?
    pub documents_mutable: bool,
    /// Can documents of this type be deleted?
    pub documents_can_be_deleted: bool,
    /// Can documents be transferred without a trade?
    pub documents_transferable: Transferable,
    /// How are these documents traded?
    pub trade_mode: TradeMode,
    /// Is document creation restricted?
    pub creation_restriction_mode: CreationRestrictionMode,
    /// The data contract id
    pub data_contract_id: Identifier,
    /// Encryption key storage requirements
    pub requires_identity_encryption_bounded_key: Option<StorageKeyRequirements>,
    /// Decryption key storage requirements
    pub requires_identity_decryption_bounded_key: Option<StorageKeyRequirements>,
    pub security_level_requirement: SecurityLevel,
    // #[cfg(feature = "validation")]
    // pub json_schema_validator: StatelessJsonSchemaLazyValidator,
}

impl DocumentTypeV0 {
    // Public method to set the data_contract_id
    pub fn set_data_contract_id(&mut self, new_id: Identifier) {
        self.data_contract_id = new_id;
    }
}
