use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;

use platform_value::{Identifier, Value};

use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::StatelessJsonSchemaLazyValidator;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

pub trait DocumentTypeV0MutGetters {
    /// Gets the schema as mut
    fn schema_mut(&mut self) -> &mut Value;
}

pub trait DocumentTypeV0Getters {
    /// Returns the name of the document type.
    fn name(&self) -> &String;

    fn schema(&self) -> &Value;

    fn schema_owned(self) -> Value;

    /// Returns the indices of the document type.
    fn indexes(&self) -> &BTreeMap<String, Index>;

    /// The contested index if one exists
    fn find_contested_index(&self) -> Option<&Index>;

    /// Returns the index structure of the document type.
    fn index_structure(&self) -> &IndexLevel;

    /// Returns the flattened properties of the document type.
    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty>;

    /// Returns the properties of the document type.
    fn properties(&self) -> &IndexMap<String, DocumentProperty>;

    /// Returns the identifier paths of the document type.
    fn identifier_paths(&self) -> &BTreeSet<String>;

    /// Returns the binary paths of the document type.
    fn binary_paths(&self) -> &BTreeSet<String>;

    /// Returns the required fields of the document type.
    fn required_fields(&self) -> &BTreeSet<String>;

    /// Returns the transient fields of the document type.
    /// Transient fields are fields that should be stripped from the document before storage.
    fn transient_fields(&self) -> &BTreeSet<String>;

    /// Returns the documents keep history flag of the document type.
    fn documents_keep_history(&self) -> bool;

    /// Returns the documents mutable flag of the document type.
    fn documents_mutable(&self) -> bool;

    /// Returns the documents can be deleted flag of the document type.
    fn documents_can_be_deleted(&self) -> bool;

    /// Returns the documents transferable flag of the document type.
    fn documents_transferable(&self) -> Transferable;

    /// Returns the documents trade mode flag of the document type.
    fn trade_mode(&self) -> TradeMode;

    /// Returns the creation restriction mode.
    fn creation_restriction_mode(&self) -> CreationRestrictionMode;

    /// Returns the data contract id of the document type.
    fn data_contract_id(&self) -> Identifier;

    /// Returns the encryption key storage requirements
    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements>;

    /// Returns the decryption key storage requirements
    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements>;

    /// The security level requirements
    fn security_level_requirement(&self) -> SecurityLevel;

    /// A reference to the json schema validator
    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator;
}

pub trait DocumentTypeV0Setters {
    /// Setting the data contract id
    fn set_data_contract_id(&mut self, data_contract_id: Identifier);
}
