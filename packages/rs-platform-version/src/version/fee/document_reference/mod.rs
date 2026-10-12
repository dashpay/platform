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
/// Every earlier schedule carries the same group; it is unread there because `chooseFirst`
/// does not parse before protocol version 14.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeDocumentReferenceVersion {
    /// The credits per competitor a `chooseFirst` may read that bound what its scan bills, and
    /// what check tx reserves for it.
    pub choose_first_credits_per_candidate: u64,
}
