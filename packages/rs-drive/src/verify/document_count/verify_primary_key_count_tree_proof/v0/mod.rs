use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentCountQuery<'_> {
    /// v0 of [`Self::verify_primary_key_count_tree_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::primary_key_count_tree_path_query`], feeds it through
    /// `GroveDb::verify_query`, and extracts `count_value_or_default()`
    /// from the verified CountTree element at `[..., doctype, 0]`.
    ///
    /// Returns 0 when the element is absent (`elements` empty or the
    /// only emitted element is `None`). The documents_countable
    /// storage layout creates the type-level CountTree at contract
    /// apply time, so absence means "no documents inserted yet", not
    /// "documents_countable is misconfigured".
    #[inline(always)]
    pub(super) fn verify_primary_key_count_tree_proof_v0(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64), Error> {
        let path_query = Self::primary_key_count_tree_path_query(contract_id, document_type_name);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        // The path query asks for exactly one key (`[0]`) under the
        // doctype path, so `elements` is either empty (CountTree
        // absent) or has a single `(path, [0], Some(CountTree))`
        // triple. Extract the count if present; 0 otherwise.
        let count = elements
            .into_iter()
            .find_map(|(_, _, elem)| elem.map(|e| e.count_value_or_default()))
            .unwrap_or(0);
        Ok((root_hash, count))
    }
}
