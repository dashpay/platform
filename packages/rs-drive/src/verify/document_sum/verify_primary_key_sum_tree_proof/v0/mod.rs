use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_primary_key_sum_tree_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::primary_key_sum_path_query`], feeds it through
    /// `GroveDb::verify_query`, and extracts `sum_value_or_default()`
    /// from the verified `SumTree` element at `[..., doctype, 1]`.
    /// Returns 0 when the element is absent.
    #[inline(always)]
    pub(super) fn verify_primary_key_sum_tree_proof_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, i64), Error> {
        let path_query = Self::primary_key_sum_path_query(contract_id, document_type_name);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        // The path query asks for exactly one key (`[1]`/SUM_TREE_KEY)
        // under the doctype path, so `elements` is either empty
        // (SumTree absent) or has a single
        // `(path, [SUM_TREE_KEY], Some(SumTree))` triple. Extract the
        // sum if present; 0 otherwise.
        let sum = elements
            .into_iter()
            .find_map(|(_, _, elem)| elem.map(|e| e.sum_value_or_default()))
            .unwrap_or(0);
        Ok((root_hash, sum))
    }
}
