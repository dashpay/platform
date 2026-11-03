mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
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
    /// document's creation paid for writing the placeholder, and the draw is free.
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
}
