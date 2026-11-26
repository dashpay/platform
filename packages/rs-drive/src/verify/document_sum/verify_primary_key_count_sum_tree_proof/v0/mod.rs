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
    ) -> Result<(RootHash, u64, i128), Error> {
        let path_query = Self::primary_key_sum_path_query(contract_id, document_type_name);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        // At 128 bits, whatever width the type keeps its sums in: for an element holding a 64-bit
        // sum (every element before protocol version 15) this is that sum widened
        // (`crate::query::sum_width`). Edited in place in this shipped generation: it returned the
        // sum at 64 bits and refused a wider one; a verifier runs on the client and decides nothing
        // in a block, and every sum it verified before is returned as the same number.
        let (count, sum) = match elements.into_iter().find_map(|(_, _, elem)| elem) {
            Some(element) => element.count_big_sum_value_or_default(),
            None => (0, 0),
        };
        Ok((root_hash, count, sum))
    }
}
