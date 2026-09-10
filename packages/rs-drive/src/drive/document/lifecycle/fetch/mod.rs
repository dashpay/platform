mod v0;

use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

/// Where one keep-history document stands in its lifecycle.
///
/// Ordinary reads cannot tell a deleted document from one that never existed —
/// both are absent from the primary-key tree — so every check that must
/// distinguish them goes through this.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentLifecycleState {
    /// The document is current and visible to ordinary reads.
    Active(Box<Document>),
    /// The document has been deleted and its revisions are retained. Carries
    /// the newest of them, which is where a deleted document's owner is read
    /// from.
    Deleted(Box<Document>),
    /// An authorized erasure has started and has not finished. Anyone may
    /// continue it, so nothing about the document's owner is needed here.
    Erasing,
    /// No document, no retained revisions and no record: the id is free.
    Absent,
}

impl DocumentLifecycleState {
    /// Whether the id is taken, which is what a create has to know.
    pub fn is_present(&self) -> bool {
        !matches!(self, DocumentLifecycleState::Absent)
    }
}

impl Drive {
    /// Classifies one keep-history document as active, deleted, erasing or
    /// absent, and returns the fee for the reads it performed.
    ///
    /// The reads are ordered by how likely they are to settle the question: the
    /// ordinary by-id read answers it for every live document, and only a miss
    /// pays for the lifecycle record. A deleted document costs one read more,
    /// for the newest revision it retains.
    ///
    /// This is deliberately not folded into the by-id fetch every document
    /// action already performs: only stateful validation of an action on a
    /// keep-history type needs the extra reads, and every other document fetch
    /// would otherwise pay for them.
    pub fn fetch_document_lifecycle(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_id: Identifier,
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(DocumentLifecycleState, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .fetch_document_lifecycle
        {
            0 => self.fetch_document_lifecycle_v0(
                contract,
                document_type,
                document_id,
                epoch,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_document_lifecycle".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
