use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::{or_empty_range_total, RootHash};
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// v1 of [`Self::verify_carrier_aggregate_count_proof`], selected from
    /// protocol version 14: v0, and no `IN` branch where grovedb refuses a
    /// proof showing the range below the carrier holds nothing (an equality
    /// value no document holds, or an empty tree of a kind the read does not
    /// aggregate), which v0 refuses as released ([`or_empty_range_total`]).
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_carrier_aggregate_count_proof_v1(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, u64)>), Error> {
        let verified = self.verify_carrier_aggregate_count_proof_v0(
            proof,
            limit,
            left_to_right,
            platform_version,
        );
        // A `summableOffCountIndex` index's range is proved by the sum surface,
        // whose verifier is at version 1 wherever this one is and has already
        // read an empty range.
        if self.index.is_summable_off_count_index() {
            return verified;
        }
        or_empty_range_total(
            verified,
            proof,
            || self.carrier_aggregate_count_path_query(limit, left_to_right, platform_version),
            |root_hash| (root_hash, Vec::new()),
            platform_version,
        )
    }
}
