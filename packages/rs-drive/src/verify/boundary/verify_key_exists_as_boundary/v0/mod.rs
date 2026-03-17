use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use grovedb::operations::proof::GroveDBProof;

impl Drive {
    pub(super) fn verify_key_exists_as_boundary_v0(
        proof: &[u8],
        path: &[&[u8]],
        key: &[u8],
    ) -> Result<bool, Error> {
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let grovedb_proof: GroveDBProof = bincode::decode_from_slice(proof, bincode_config)
            .map(|(p, _)| p)
            .map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode GroveDBProof: {}",
                    e
                )))
            })?;

        grovedb_proof
            .key_exists_as_boundary(path, key)
            .map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "error checking boundary key: {}",
                    e
                )))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use grovedb::batch::QualifiedGroveDbOp;
    use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery};
    use platform_version::version::PlatformVersion;

    /// Helper: insert items with keys [1..=count] under a test subtree within
    /// the existing Drive tree structure. Uses DataContractDocuments root tree
    /// as the parent and creates a subtree "test" beneath it.
    fn insert_test_items(drive: &Drive, count: u8, platform_version: &PlatformVersion) {
        use crate::drive::RootTree;
        use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use crate::util::batch::GroveDbOpBatch;

        // Create the test subtree under DataContractDocuments
        let parent_path = vec![vec![RootTree::DataContractDocuments as u8]];
        let subtree_op = QualifiedGroveDbOp::insert_or_replace_op(
            parent_path.clone(),
            b"test".to_vec(),
            Element::empty_tree(),
        );
        drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(vec![subtree_op]),
                false,
                None,
                &platform_version.drive,
            )
            .expect("should create test subtree");

        // Insert items
        let test_path = vec![
            vec![RootTree::DataContractDocuments as u8],
            b"test".to_vec(),
        ];
        let ops: Vec<_> = (1..=count)
            .map(|i| {
                QualifiedGroveDbOp::insert_or_replace_op(
                    test_path.clone(),
                    vec![i],
                    Element::new_item(vec![i; 4]),
                )
            })
            .collect();
        drive
            .grove_apply_batch(
                GroveDbOpBatch::from_operations(ops),
                false,
                None,
                &platform_version.drive,
            )
            .expect("should insert items");
    }

    fn test_path_refs() -> Vec<Vec<u8>> {
        use crate::drive::RootTree;
        vec![
            vec![RootTree::DataContractDocuments as u8],
            b"test".to_vec(),
        ]
    }

    #[test]
    fn should_detect_boundary_key_in_range_after_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert keys [1, 2, 3, 4, 5]
        insert_test_items(&drive, 5, platform_version);

        // Query: RangeAfter(key=3) — key 3 is the boundary, results are [4, 5]
        let path_vecs = test_path_refs();
        let mut query = Query::new();
        query.insert_item(QueryItem::RangeAfter(vec![3u8]..));
        let path_query = PathQuery::new(
            path_vecs.clone(),
            SizedQuery {
                query,
                limit: Some(10),
                offset: None,
            },
        );

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Key 3 should exist as a boundary
        let path_refs: Vec<&[u8]> = path_vecs.iter().map(|v| v.as_slice()).collect();
        let exists =
            Drive::verify_key_exists_as_boundary(&proof, &path_refs, &[3u8], platform_version)
                .expect("should verify");
        assert!(exists, "key 3 should exist as boundary in RangeAfter proof");
    }

    #[test]
    fn should_not_detect_non_existent_key_as_boundary() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        insert_test_items(&drive, 5, platform_version);

        let path_vecs = test_path_refs();
        let mut query = Query::new();
        query.insert_item(QueryItem::RangeAfter(vec![3u8]..));
        let path_query = PathQuery::new(
            path_vecs.clone(),
            SizedQuery {
                query,
                limit: Some(10),
                offset: None,
            },
        );

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Key 99 was never inserted — should not be a boundary
        let path_refs: Vec<&[u8]> = path_vecs.iter().map(|v| v.as_slice()).collect();
        let exists =
            Drive::verify_key_exists_as_boundary(&proof, &path_refs, &[99u8], platform_version)
                .expect("should verify");
        assert!(
            !exists,
            "non-existent key should not be detected as boundary"
        );
    }

    #[test]
    fn should_not_detect_result_key_as_boundary() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        insert_test_items(&drive, 5, platform_version);

        let path_vecs = test_path_refs();
        let mut query = Query::new();
        // RangeFrom is inclusive — key 3 is a result, not a boundary
        query.insert_item(QueryItem::RangeFrom(vec![3u8]..));
        let path_query = PathQuery::new(
            path_vecs.clone(),
            SizedQuery {
                query,
                limit: Some(10),
                offset: None,
            },
        );

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof");

        // Key 3 is a result in RangeFrom (inclusive), not a boundary
        let path_refs: Vec<&[u8]> = path_vecs.iter().map(|v| v.as_slice()).collect();
        let exists =
            Drive::verify_key_exists_as_boundary(&proof, &path_refs, &[3u8], platform_version)
                .expect("should verify");
        assert!(
            !exists,
            "inclusive range start key is a result, not a boundary"
        );
    }

    #[test]
    fn should_reject_invalid_proof_bytes() {
        let platform_version = PlatformVersion::latest();
        let result = Drive::verify_key_exists_as_boundary(
            &[0xff, 0xfe],
            &[b"test"],
            &[1u8],
            platform_version,
        );
        assert!(result.is_err(), "garbage proof bytes should return error");
    }
}
