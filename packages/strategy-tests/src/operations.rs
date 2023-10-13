use crate::frequency::Frequency;
use dpp::data_contract::document_type::random_document::{
    DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::document_type::v0::random_document_type::RandomDocumentTypeParameters;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DataContract as Contract;
use dpp::identifier::Identifier;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::Value;
use std::collections::BTreeMap;
use std::ops::Range;

#[derive(Clone, Debug)]
pub enum DocumentAction {
    DocumentActionInsertRandom(DocumentFieldFillType, DocumentFieldFillSize),
    /// Insert a document with specific values
    /// If a required value is not set, it will use random ones
    /// The second parameter is the owner id of the document
    /// If none then it should be random
    DocumentActionInsertSpecific(
        BTreeMap<String, Value>,
        Option<Identifier>,
        DocumentFieldFillType,
        DocumentFieldFillSize,
    ),
    DocumentActionDelete,
    DocumentActionReplace,
}

#[derive(Clone, Debug)]
pub struct DocumentOp {
    pub contract: Contract,
    pub document_type: DocumentType,
    pub action: DocumentAction,
}

#[derive(Clone, Debug)]
pub struct Operation {
    pub op_type: OperationType,
    pub frequency: Frequency,
}

#[derive(Clone, Debug)]
pub enum IdentityUpdateOp {
    IdentityUpdateAddKeys(u16),
    IdentityUpdateDisableKey(u16),
}

pub type DocumentTypeNewFieldsOptionalCountRange = Range<u16>;
pub type DocumentTypeCount = Range<u16>;

#[derive(Clone, Debug)]
pub enum DataContractUpdateOp {
    DataContractNewDocumentTypes(RandomDocumentTypeParameters), // How many fields should it have
    DataContractNewOptionalFields(DocumentTypeNewFieldsOptionalCountRange, DocumentTypeCount), // How many new fields on how many document types
}

#[derive(Clone, Debug)]
pub enum OperationType {
    Document(DocumentOp),
    IdentityTopUp,
    IdentityUpdate(IdentityUpdateOp),
    IdentityWithdrawal,
    ContractCreate(RandomDocumentTypeParameters, DocumentTypeCount),
    ContractUpdate(DataContractUpdateOp),
    IdentityTransfer,
}

#[derive(Clone, Debug)]
pub enum FinalizeBlockOperation {
    IdentityAddKeys(Identifier, Vec<IdentityPublicKey>),
}
