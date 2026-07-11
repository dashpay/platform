use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_primary_key_count_sum_tree_proof`].
    ///
    /// Same single-key `verify_query` shape as
    /// [`Self::verify_primary_key_sum_tree_proof_v0`], but extracts
    /// `count_sum_value_or_default()` to recover `(count, sum)` from
    /// any of the count-sum-bearing tree variants
    /// (`CountSumTree` / `ProvableCountSumTree` /
    /// `ProvableCountProvableSumTree`). Returns `(0, 0)` when the
    /// element is absent.
    #[inline(always)]
    pub(super) fn verify_primary_key_count_sum_tree_proof_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64, i64), Error> {
        let path_query = Self::primary_key_sum_path_query(contract_id, document_type_name);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let (count, sum) = elements
            .into_iter()
            .find_map(|(_, _, elem)| elem.map(|e| e.count_sum_value_or_default()))
            .unwrap_or((0, 0));
        Ok((root_hash, count, sum))
    }
}
