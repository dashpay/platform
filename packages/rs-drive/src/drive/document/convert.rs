use dpp::prelude::Document;

use crate::error::{drive::DriveError, Error};

/// Helper function to convert DPP documents to Drive documents
pub fn convert_dpp_documents_to_drive_documents<'a, I>(
    dpp_documents: I,
) -> Result<Vec<crate::contract::document::Document>, Error>
where
    I: Iterator<Item = &'a Document>,
{
    dpp_documents
        .map(|document| {
            crate::contract::document::Document::from_cbor(
                &document.to_cbor().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't convert dpp document to cbor",
                    ))
                })?,
                None,
                None,
            )
        })
        .collect::<Result<Vec<crate::contract::document::Document>, Error>>()
}
