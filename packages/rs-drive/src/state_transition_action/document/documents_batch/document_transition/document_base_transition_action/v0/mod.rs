/// transformer
pub mod transformer;

use std::sync::Arc;

use dpp::identifier::Identifier;

use crate::drive::contract::DataContractFetchInfo;

#[derive(Debug, Clone)]
/// document base transition action v0
pub struct DocumentBaseTransitionActionV0 {
    /// The document Id
    pub id: Identifier,
    /// Name of document type found int the data contract associated with the `data_contract_id`
    pub document_type_name: String,
    /// A potential data contract
    pub data_contract: Arc<DataContractFetchInfo>,
}

/// document base transition action accessors v0
pub trait DocumentBaseTransitionActionAccessorsV0 {
    /// The document Id
    fn id(&self) -> Identifier;
    /// Name of document type found int the data contract associated with the `data_contract_id`
    fn document_type_name(&self) -> &String;
    /// document type name owned
    fn document_type_name_owned(self) -> String;
    /// Data contract ID generated from the data contract's `owner_id` and `entropy`
    fn data_contract_id(&self) -> Identifier;
    /// Data contract
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo>;
}
