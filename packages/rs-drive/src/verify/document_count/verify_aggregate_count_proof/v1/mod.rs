use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::{or_empty_range_total, RootHash};
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// v1 of [`Self::verify_aggregate_count_proof`], selected from protocol
    /// version 14: v0, and a zero total where grovedb refuses a proof showing
    /// the range holds nothing (an equality value no document holds, or an
    /// empty tree of a kind the read does not aggregate), which v0 refuses as
    /// released ([`or_empty_range_total`]).
    #[inline(always)]
    pub(super) fn verify_aggregate_count_proof_v1(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64), Error> {
        let verified = self.verify_aggregate_count_proof_v0(proof, platform_version);
        // A `summableOffCountIndex` index's range is proved by the sum surface,
        // whose verifier is at version 1 wherever this one is and has already
        // read an empty range.
        if self.index.is_summable_off_count_index() {
            return verified;
        }
        or_empty_range_total(
            verified,
            proof,
            || self.aggregate_count_path_query(platform_version),
            |root_hash| (root_hash, 0),
            platform_version,
        )
    }
}
