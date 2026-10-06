use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::{or_empty_range_total, RootHash};
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// v1 of [`Self::verify_aggregate_sum_proof`], selected from protocol
    /// version 14: v0, and a zero sum where grovedb refuses a proof showing the
    /// range holds nothing (an equality value no document holds, or an empty
    /// tree of a kind the read does not aggregate), which v0 refuses as
    /// released ([`or_empty_range_total`]).
    #[inline(always)]
    pub(super) fn verify_aggregate_sum_proof_v1(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, i64), Error> {
        let verified = self.verify_aggregate_sum_proof_v0(proof, platform_version);
        or_empty_range_total(
            verified,
            proof,
            || self.aggregate_sum_path_query(platform_version),
            |root_hash| (root_hash, 0),
            platform_version,
        )
    }
}
