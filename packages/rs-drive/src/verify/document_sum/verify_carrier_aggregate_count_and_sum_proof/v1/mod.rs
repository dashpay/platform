use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::{or_empty_range_total, RootHash};
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// v1 of [`Self::verify_carrier_aggregate_count_and_sum_proof`], selected
    /// from protocol version 14: v0, and no `IN` branch where grovedb refuses a
    /// proof showing the range below the carrier holds nothing (an equality
    /// value no document holds, or an empty tree of a kind the read does not
    /// aggregate), which v0 refuses as released ([`or_empty_range_total`]).
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_carrier_aggregate_count_and_sum_proof_v1(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, u64, i64)>), Error> {
        let verified = self.verify_carrier_aggregate_count_and_sum_proof_v0(
            proof,
            limit,
            left_to_right,
            platform_version,
        );
        or_empty_range_total(
            verified,
            proof,
            || {
                self.carrier_aggregate_count_and_sum_path_query(
                    limit,
                    left_to_right,
                    platform_version,
                )
            },
            |root_hash| (root_hash, Vec::new()),
            platform_version,
        )
    }
}
