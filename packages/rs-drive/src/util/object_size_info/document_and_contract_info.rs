use crate::util::object_size_info::OwnedDocumentInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

/// Document and contract info
#[derive(Clone, Debug)]
pub struct DocumentAndContractInfo<'a> {
    /// Document info
    pub owned_document_info: OwnedDocumentInfo<'a>,
    ///DataContract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
}

impl DocumentAndContractInfo<'_> {
    /// The same write without storage flags when the document type declares a `ttl`: such a
    /// document is stored flagless and refunds nothing. Every other write is unchanged. Done
    /// before any element is sized, so sizes, replacements and patches all agree with what is
    /// stored.
    pub fn without_storage_flags_if_expiring(self) -> Self {
        if self.document_type.documents_ttl_seconds().is_none() {
            return self;
        }
        let DocumentAndContractInfo {
            owned_document_info,
            contract,
            document_type,
        } = self;
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: owned_document_info.document_info.without_storage_flags(),
                owner_id: owned_document_info.owner_id,
            },
            contract,
            document_type,
        }
    }

    /// Whether the trees of this document's indexes carry its storage flags, which its
    /// primary storage and its index references always keep. They do not for an immutable
    /// type of a contract that cannot be deleted: nothing ever removes those trees for a
    /// refund. The insert walker applies this, and a draw of a `sys.random` property follows
    /// it, so a drawn document's index trees are the ones its create priced.
    pub(crate) fn index_trees_carry_storage_flags(&self) -> bool {
        self.document_type.documents_mutable()
            || self.contract.config().can_be_deleted()
            // indexOnly entries ARE the rows: they are deleted (and refunded) whenever the
            // doctype allows deletion, so their flags must ride even though the type is
            // immutable and the contract may not be deletable. PV14-only (`index_only()`
            // cannot be true below meta-schema v3), so historical replay is untouched.
            || (self.document_type.index_only() && self.document_type.documents_can_be_deleted())
    }
}
