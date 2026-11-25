//! The grouped mode a count, sum or average request is answered in, decided
//! by Drive's routing of the request
//! ([`compute_aggregate_mode_and_check_limit`]), the routing every node runs
//! before it executes one, at the protocol version the request is encoded
//! for.

use crate::documents::document_query::{having_page_refusal, DocumentQuery};
use dpp::version::PlatformVersion;
use drive::query::aggregate_routing::{compute_aggregate_mode_and_check_limit, AggregateRouting};
use drive::query::CountMode;

/// The grouped mode a node answers `request` in, a `function_name`
/// (`"COUNT"`, `"SUM"` or `"AVG"`) select with its pending windows resolved.
///
/// The routing refuses what every node refuses — a grouping no `In` or
/// range constrains, a `limit` the mode cannot honor — in the node's words.
/// A request it routes to the ranked or having-range surface is answered
/// with a page a count, sum or average view cannot verify, so it is refused
/// in favour of the view that verifies it.
pub(super) fn grouped_mode(
    request: &DocumentQuery,
    function_name: &str,
    platform_version: &PlatformVersion,
) -> Result<CountMode, drive_proof_verifier::Error> {
    let routing = compute_aggregate_mode_and_check_limit(
        &request.select,
        &request.group_by,
        &request.where_clauses,
        &request.order_by_clauses,
        (request.limit != 0).then_some(request.limit),
        &request.having,
        function_name,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!("no node answers this {function_name} request: {e}"),
    })?;
    match routing {
        AggregateRouting::Grouped(mode) => Ok(mode),
        AggregateRouting::Ranked => Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "a grouped {function_name} ordered by itself is answered as a ranked page, which \
                 a count, sum or average view cannot verify; fetch it as DocumentRankedEntries"
            ),
        }),
        AggregateRouting::HavingRange => Err(having_page_refusal()),
    }
}
