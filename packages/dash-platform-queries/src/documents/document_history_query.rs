//! Query type for retrieving document history.

use dpp::prelude::Identifier;
pub use drive::drive::document::history::DocumentHistorySelector;

/// Query parameters for a document's historical revisions.
#[derive(Debug, Clone, PartialEq, Eq, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentHistoryQuery {
    /// Data contract ID.
    pub data_contract_id: Identifier,
    /// Document type name within the data contract.
    pub document_type_name: String,
    /// Document ID.
    pub document_id: Identifier,
    /// Exactly one time cursor or revision selector.
    pub selector: DocumentHistorySelector,
    /// Maximum number of history entries to return.
    pub limit: Option<u32>,
}
