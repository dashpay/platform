use bincode::{Decode, Encode};

pub mod v1;

/// Fees of the reads a document reference (`refersTo`) makes when a document is written
/// (protocol version 14).
///
/// A create whose `refersTo` declares `chooseFirst` reads the competitors of the commitment it
/// found, at most `SystemLimits::max_choose_first_candidates` documents in one query, and hashes
/// the `findBy` function once for each. Both are billed as they run, but never more than
/// `choose_first_credits_per_candidate` times that limit, the scan's budget. Check tx, which
/// does not validate such a create's state, holds its signer to the whole budget on top of the
/// create's estimated fee, so a block never charges the scan more than the signer was admitted
/// with: a signer who could not pay for a full scan is kept out of the mempool rather than
/// refused unpaid in a block, with its nonce left to replay.
///
/// A `where` entry comparing typed arrays (an equality between two of them, `contains` or
/// `containsAll`) is billed by the list elements it reads, before it compares them.
///
/// A reference naming its target through the referring document judges the named document
/// type against its `documentTypeRequirements` on every write that checks it. Nothing is
/// read from state for it, but the work grows with the type's properties, the slots and
/// their `when` conditions, so it is billed by the comparisons it may take.
///
/// Every earlier schedule carries the same group; it is unread there because neither
/// `chooseFirst` nor a `where` entry parses before protocol version 14, and no reference
/// names its target through the referring document before protocol version 15.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeDocumentReferenceVersion {
    /// The credits per competitor a `chooseFirst` may read that bound what its scan bills, and
    /// what check tx reserves for it.
    pub choose_first_credits_per_candidate: u64,
    /// The credits per list element a `where` entry comparing typed arrays reads, on both
    /// sides, against a document in state or a moderated document's removal record.
    pub where_list_credits_per_element: u64,
    /// The credits per comparison judging the document type a reference names through the
    /// referring document against its `documentTypeRequirements` takes
    /// (`DocumentTypeRequirements::comparisons`): each property a slot tries, and each node
    /// of the slot's `when` for it. Billed before the type is judged, so a write refused
    /// for a type that does not meet them pays for it too (protocol version 15).
    pub requirement_credits_per_comparison: u64,
}
