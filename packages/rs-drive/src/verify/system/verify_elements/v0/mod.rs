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
    // TODO: Use type alias or struct
    #[allow(clippy::type_complexity)]
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

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_and_verify_elements() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // The initial state structure creates various root trees.
        // We will prove and verify elements at a known path.
        // The balances root tree is at path [RootTree::Balances].
        // After initial setup, the "misc" tree has the total system credits key.
        let path = vec![vec![crate::drive::RootTree::Misc as u8]];
        let keys = vec![crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec()];

        let proof = drive
            .prove_elements(path.clone(), keys.clone(), None, platform_version)
            .expect("should generate proof for elements");

        let (_root_hash, verified_elements) =
            Drive::verify_elements(&proof, path, keys.clone(), platform_version)
                .expect("should verify elements proof");

        // We should get back exactly one entry for the key we queried
        assert_eq!(verified_elements.len(), 1);
        let key = &keys[0];
        assert!(
            verified_elements.contains_key(key),
            "verified elements should contain the requested key"
        );
        // The element should exist (it's initialized during setup)
        assert!(
            verified_elements[key].is_some(),
            "the total system credits element should exist"
        );
    }
}
