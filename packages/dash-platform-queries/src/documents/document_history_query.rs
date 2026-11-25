//! Query type for retrieving document history.

use crate::Error;
use dapi_grpc::platform::v0::get_document_history_request::get_document_history_request_v1::Filter;
use dapi_grpc::platform::v0::get_document_history_request::{GetDocumentHistoryRequestV0, Version};
use dapi_grpc::platform::v0::GetDocumentHistoryRequest;
use dpp::prelude::Identifier;
use drive::error::query::QuerySyntaxError;
use drive::error::Error as DriveError;
use drive::query::document_history_drive_query::DocumentHistoryDriveQuery;
pub use drive::query::document_history_drive_query::DocumentHistoryFilter;

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
    /// Exactly one time cursor or revision filter.
    pub filter: DocumentHistoryFilter,
    /// Maximum number of history entries to return.
    pub limit: Option<u32>,
}

impl DocumentHistoryQuery {
    /// The filter of a history read that names no starting point: the first page, from the
    /// first retained revision. It is a start time, the one filter every protocol version
    /// serves ([`history_request_below_protocol_15`]).
    ///
    /// A binding whose input may leave the filter out uses this instead of choosing a
    /// starting point of its own.
    pub const FIRST_PAGE_FILTER: DocumentHistoryFilter = DocumentHistoryFilter::StartAtTime(0);
}

/// A history request Drive refuses as written, as Drive's own query validation reports it.
fn malformed(message: &str) -> Error {
    Error::Drive(DriveError::Query(QuerySyntaxError::InvalidParameter(
        message.to_owned(),
    )))
}

/// The V0 form of a V1 history request, which a network below protocol version 15 serves:
/// a start-time filter, the only one V0 has, starting one millisecond earlier. A start time
/// of 0 asks V0 for everything after time 0, the earliest it can ask from.
///
/// `Ok(None)` for a valid revision or cursor filter, which has no V0 form. A request V1
/// itself refuses is an error, as Drive's history query validation reports it
/// ([`Error::Drive`] with a query error): one with no filter, and a start time or limit out
/// of range, which in V0 would select another page instead of being refused. A request
/// that is not V1 is returned as it is.
///
/// Whether to send V0 is the SDK's decision. This conversion is also how a caller derives
/// the request it expects when it verifies a V0 history proof it did not fetch.
pub fn history_request_below_protocol_15(
    request: GetDocumentHistoryRequest,
) -> Result<Option<GetDocumentHistoryRequest>, Error> {
    let Some(Version::V1(v1)) = request.version else {
        return Ok(Some(request));
    };
    // Every filter is validated as V1 first, so `Ok(None)` means only a valid filter with no
    // V0 form, and a request V1 refuses too is reported as such.
    let filter = match &v1.filter {
        None => return Err(malformed("a history request needs exactly one filter")),
        Some(Filter::StartAtMs(time)) => DocumentHistoryFilter::StartAtTime(*time),
        Some(Filter::StartAfter(cursor)) => DocumentHistoryFilter::StartAfter {
            time_ms: cursor.time_ms,
            revision: cursor.revision,
        },
        Some(Filter::StartAtRevision(revision)) => {
            DocumentHistoryFilter::StartAtRevision(*revision)
        }
        Some(Filter::Revision(revision)) => DocumentHistoryFilter::Revision(*revision),
    };
    let identifier = |bytes: &[u8]| {
        <[u8; 32]>::try_from(bytes).map_err(|_| malformed("history identifiers are 32 bytes"))
    };
    DocumentHistoryDriveQuery {
        contract_id: identifier(&v1.data_contract_id)?,
        document_type_name: v1.document_type_name.clone(),
        document_id: identifier(&v1.document_id)?,
        filter: filter.clone(),
        limit: v1
            .limit
            .map(u16::try_from)
            .transpose()
            .map_err(|_| malformed("history limit out of bounds"))?,
    }
    .validate()?;
    let DocumentHistoryFilter::StartAtTime(start_at_ms) = filter else {
        return Ok(None);
    };
    Ok(Some(
        GetDocumentHistoryRequestV0 {
            data_contract_id: v1.data_contract_id,
            document_type_name: v1.document_type_name,
            document_id: v1.document_id,
            limit: v1.limit,
            offset: None,
            // V1 starts at its time and V0 after it (Drive's `start_after_ms`), so V0 starts
            // one millisecond earlier. V0 cannot start at time 0 itself, so 0 stays 0, which
            // asks for everything after time 0. A revision's time is its block's time, in
            // milliseconds since the Unix epoch, so a network holds no revision at time 0;
            // only a test that writes with a default block time does.
            start_at_ms: start_at_ms.saturating_sub(1),
            prove: v1.prove,
        }
        .into(),
    ))
}
