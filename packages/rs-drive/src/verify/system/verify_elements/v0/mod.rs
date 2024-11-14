use crate::drive::Drive;
use crate::error::Error;
use crate::query::Query;
use crate::verify::RootHash;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::{Element, GroveDb, PathQuery, SizedQuery};
use grovedb_version::TryIntoVersioned;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies a proof and returns elements matching keys for a path.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `path`: The path where elements should be.
    /// - `keys`: The requested keys.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `BTreeMap<Vec<u8>, Option<Element>>`. The `BTreeMap<Vec<u8>, Option<Element>>`
    /// represent the elements we were trying to fetch.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(super) fn verify_elements_v0(
        proof: &[u8],
        path: Vec<Vec<u8>>,
        keys: Vec<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, BTreeMap<Vec<u8>, Option<Element>>), Error> {
        let mut query = Query::new();
        query.insert_keys(keys);
        let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));

        let (root_hash, proved_path_key_values) =
            GroveDb::verify_query_raw(proof, &path_query, &platform_version.drive.grove_version)?;
        let path_key_optional_elements = proved_path_key_values
            .into_iter()
            .map(|pkv| {
                let key_element_pair: PathKeyOptionalElementTrio =
                    pkv.try_into_versioned(&platform_version.drive.grove_version)?;
                Ok((key_element_pair.1, key_element_pair.2))
            })
            .collect::<Result<BTreeMap<Vec<u8>, Option<Element>>, grovedb::Error>>()?;
        Ok((root_hash, path_key_optional_elements))
    }
}
