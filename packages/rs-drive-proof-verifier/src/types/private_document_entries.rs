//! A page of the entries of a private document type's store (`getPrivateDocumentEntries`,
//! protocol version 15): the type's fixed-size encrypted entries in append order, each with
//! the height and time of the block that appended it, and how many entries the store holds.
//!
//! Both are Drive's own types, which its proof verifier returns.

use crate::Error;
use dapi_grpc::platform::v0::get_private_document_entries_response::get_private_document_entries_response_v0::PrivateDocumentEntries as PrivateDocumentEntriesProto;
pub use drive::drive::document::private_document_store::{
    PrivateDocumentEntries, PrivateDocumentEntry,
};

/// The page of an unproved response, held to the page the request asked for: its entries
/// start at `start_position`, their positions are consecutive and below the store's count,
/// there are at most `count` of them when `count` is not 0, and the page is empty only when
/// `start_position` is at or past the store's end. Without a proof this authenticates
/// nothing; it refuses a response that cannot be the page asked for.
pub(crate) fn private_document_entries_from_response(
    page: PrivateDocumentEntriesProto,
    start_position: u64,
    count: u32,
) -> Result<PrivateDocumentEntries, Error> {
    let total_count = page.total_count;
    if count != 0 && page.entries.len() > count as usize {
        return Err(Error::ResponseDecodeError {
            error: format!(
                "{} private document entries returned, the request asked for at most {count}",
                page.entries.len()
            ),
        });
    }
    if page.entries.is_empty() && start_position < total_count {
        return Err(Error::ResponseDecodeError {
            error: format!(
                "no private document entries returned from position {start_position} of a store \
                 of {total_count}"
            ),
        });
    }
    let entries = page
        .entries
        .into_iter()
        .zip(0u64..)
        .map(|(entry, index)| {
            // A saturated position cannot pass the count check below
            let expected_position = start_position.saturating_add(index);
            if entry.position != expected_position || entry.position >= total_count {
                return Err(Error::ResponseDecodeError {
                    error: format!(
                        "private document entry {index} of a page from position \
                         {start_position} is at position {}, not {expected_position} below the \
                         store's count {total_count}",
                        entry.position
                    ),
                });
            }
            Ok(PrivateDocumentEntry {
                position: entry.position,
                block_height: entry.block_height,
                block_time_ms: entry.block_time_ms,
                data: entry.data,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(PrivateDocumentEntries {
        entries,
        total_count,
    })
}
