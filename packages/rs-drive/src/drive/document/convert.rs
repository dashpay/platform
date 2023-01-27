use dpp::{document::document_stub::DocumentStub, prelude::Document};

use crate::error::{drive::DriveError, Error};

/// Helper function to convert DPP documents to Drive documents
pub fn convert_dpp_documents_to_drive_documents<'a, I>(
    dpp_documents: I,
) -> Result<Vec<DocumentStub>, Error>
where
    I: Iterator<Item = &'a Document>,
{
    dpp_documents
        .map(|document| {
            DocumentStub::from_cbor(
                &document.to_buffer().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't convert dpp document to CBOR",
                    ))
                })?,
                None,
                None,
            )
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create drive document from CBOR",
                ))
            })
        })
        .collect::<Result<Vec<DocumentStub>, Error>>()
}
