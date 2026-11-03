mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// A document a block created, whose `sys.random` properties the block's random beacon fills in.
/// Recorded for every document a block creates; the draw reads the document's type, and leaves
/// alone one that declares no such property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomDocumentRequest {
    /// The contract defining the document's type.
    pub contract_id: Identifier,
    /// The document's type.
    pub document_type_name: String,
    /// The document.
    pub document_id: Identifier,
}

impl Drive {
    /// Fills each `sys.random` property of the documents `requests` names with a value drawn
    /// from `beacon`, the block's random beacon (the threshold signature the block's prevotes
    /// combine into), once every transition of the block is accepted. Each document was written
    /// with the placeholder, its range's minimum (`generate_random_properties`), and is
    /// rewritten in place: its primary storage, and each index entry the drawn value moves.
    /// A document a later transition of the block deleted is skipped. Nobody pays: the
    /// document's creation paid for writing the placeholder, and prepaid the index entries the
    /// draw may create ([`Self::prepay_random_document_index_entries`]), so the draw is free.
    ///
    /// # Parameters
    /// - `beacon`: the block's random beacon.
    /// - `requests`: the documents the block created, in the order the block created them.
    /// - `block_info`: the block.
    /// - `transaction`: the block's transaction.
    /// - `platform_version`: selects the method version; `None` before protocol version 15,
    ///   whose parser is the first to admit a `sys.random` property.
    ///
    /// # Returns
    /// How many documents had properties drawn.
    pub fn draw_random_document_properties(
        &self,
        beacon: &[u8],
        requests: &[RandomDocumentRequest],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u32, Error> {
        match platform_version
            .drive
            .methods
            .document
            .draw_random_document_properties
        {
            Some(0) => self.draw_random_document_properties_v0(
                beacon,
                requests,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "draw_random_document_properties".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "draw_random_document_properties".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// What a create of `document`, of `document_type` in `contract`, written by `owner_id`,
    /// pays in advance for the draw of its `sys.random` properties
    /// ([`Self::draw_random_document_properties`]). The draw moves the document's entry in each
    /// index holding such a property from the placeholder's value to the drawn one, and the
    /// branch it moves into may not exist yet: the draw creates it, its elements carrying the
    /// document's storage flags, so its owner is refunded for them on delete, and the draw
    /// itself is not billed. So the create pays for those entries once more, at what Drive's
    /// fee estimation gives for inserting the document's entries into those indexes without
    /// reading state: the average layer information it prices a dry run with, as if the
    /// branches had to be created (the average case, whatever the draw finds). Selected by the
    /// draw's method version, whose cost model it is part of.
    ///
    /// # Parameters
    /// - `document`: the document the create writes, holding the placeholders.
    /// - `owner_id`: its owner, whose storage flags the entries carry.
    /// - `contract`: the contract defining its type.
    /// - `document_type`: its type.
    /// - `block_info`: the block, whose epoch prices the entries.
    /// - `transaction`: the block's transaction.
    /// - `platform_version`: selects the method version; `None` before protocol version 15,
    ///   whose parser is the first to admit a `sys.random` property.
    ///
    /// # Returns
    /// The fee of those entries; zero when no index of the type holds such a property.
    #[allow(clippy::too_many_arguments)]
    pub fn prepay_random_document_index_entries(
        &self,
        document: &Document,
        owner_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .draw_random_document_properties
        {
            Some(0) => self.prepay_random_document_index_entries_v0(
                document,
                owner_id,
                contract,
                document_type,
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prepay_random_document_index_entries".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "prepay_random_document_index_entries".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
