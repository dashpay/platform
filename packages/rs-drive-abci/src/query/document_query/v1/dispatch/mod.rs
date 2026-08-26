//! Per-route executors for the v1 `getDocuments` handler — one file
//! per `RoutingDecision` arm, split from the handler's `mod.rs` for
//! readability. Each file holds one `impl<C> Platform<C>` block with
//! that route's dispatcher; helpers shared by exactly one dispatcher
//! live next to it, and helpers shared by the ranked + having pair
//! (which reuse the same wire entry shape) live here.

mod average;
mod count;
mod documents;
mod having;
mod ranked;
mod sum;

use crate::error::query::QueryError;
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    ranked_entry, RankedEntry,
};
use drive::query::{RankedEntry as DriveRankedEntry, RankedEntryValue};

/// Translate an rs-drive `RankedEntry` into the wire `RankedEntry`.
/// Mirror of [`into_v1_entry`] / [`into_v1_sum_entry`] /
/// [`into_v1_average_entry`] for the ranked surface.
///
/// `key` passes through as the raw index-key bytes of the grouped
/// property value — the same bytes the proof commits to, so a client
/// verifying the proof compares byte-for-byte against what it
/// reconstructs.
///
/// The `value` oneof is always set: drive's `RankedEntryValue` has no
/// "absent" variant (unlike the count / sum entry types, whose
/// `Option` exists for the SDK's synthesize-for-missing-In-value
/// concept — a ranked result has no caller-supplied key set to be
/// silent about).
fn into_v1_ranked_entry(e: DriveRankedEntry) -> RankedEntry {
    RankedEntry {
        // Set exactly when the request carried an `IN` prefix pin —
        // drive's merge tags entries with their branch, single-branch
        // responses stay untagged.
        in_key: e.in_key,
        key: e.key,
        value: Some(match e.value {
            RankedEntryValue::Count(count) => ranked_entry::Value::Count(count),
            RankedEntryValue::Sum(sum) => ranked_entry::Value::Sum(sum),
            // The wire carries a `double` approximation of the exact
            // fixed-point `i128` the Avg axis is ordered by:
            // `fixed_point as f64 / RANKED_AVG_SCALE as f64`, which is
            // what `as_f64` computes. Lossy by construction, and that
            // is fine — these entries are only read on the no-proof
            // ("quick answer") path. A proof-verifying client ignores
            // this field and reconstructs the exact fixed point from
            // the grovedb proof, so no verification depends on it.
            // Ranking order is still exact: the ordering happened over
            // the i128 before this conversion.
            value @ RankedEntryValue::AvgFixedPoint(_) => ranked_entry::Value::Avg(value.as_f64()),
        }),
    }
}

/// Recognize the one grovedb failure that is a caller-facing
/// condition rather than a server fault: **an empty ranking cannot be
/// proved**.
///
/// **This is now a backstop rather than a live path.** The ranked
/// prover moved to the paginated axis traversal (today through
/// grovedb's unified `prove_query`), which emits a guaranteed-empty
/// range against an empty axis secondary instead of refusing, so
/// proving a ranking over a contract with no documents succeeds and
/// the proved and unproven paths agree (pinned by
/// `ranked_tests::proving_an_empty_ranking_succeeds`). The mapping is
/// kept because the failure it recognizes is a *class* — a merk-level
/// "cannot prove an empty tree" surfacing from somewhere in the
/// ancestor chain — not a single call site, and because the cost of
/// keeping it is one string comparison on an error path.
///
/// Historically: the non-paginated prover had no absence-proof shape
/// for "this axis secondary has no entries", so proving a ranking over
/// an index that held no documents failed with a merk-level "Cannot
/// create proof for empty tree", wrapped by grovedb as
/// `CorruptedData`. Reaching that state needed nothing exotic —
/// querying a freshly registered contract with `prove = true` did it —
/// so letting it propagate would answer an ordinary request with an
/// internal error (`Status::unknown`) and an alarming server-side log
/// line, and give the caller no idea that the same request without
/// `prove` succeeded and returned the empty list.
///
/// Detection is by variant + marker substring rather than by a typed
/// error, because grovedb flattens the merk error into a
/// `CorruptedData(String)` at the indexed-axis proof boundary; the
/// substring is the merk-side constant. The match is deliberately
/// narrow: any other `CorruptedData` still propagates as an internal
/// error, because for every other cause that classification is
/// correct.
fn empty_ranking_proof_rejection(error: &drive::error::Error) -> Option<QueryError> {
    let drive::error::Error::GroveDB(grove_error) = error else {
        return None;
    };
    let drive::query::GroveError::CorruptedData(message) = grove_error.as_ref() else {
        return None;
    };
    if !message.contains("Cannot create proof for empty tree") {
        return None;
    }
    Some(QueryError::InvalidArgument(
        "this index's axis secondary has no groups yet, and an empty ranking or \
         HAVING range cannot be proved: grovedb has no absence-proof shape for \
         an empty axis secondary. Retry with `prove = false` — the unproven \
         read answers the same request with an empty entry list. Once the \
         index holds at least one document, the proved form works."
            .to_string(),
    ))
}
