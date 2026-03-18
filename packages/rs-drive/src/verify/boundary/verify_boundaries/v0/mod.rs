use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use grovedb::operations::proof::GroveDBProof;

impl Drive {
    pub(super) fn verify_boundaries_v0(
        proof: &[u8],
        path: &[&[u8]],
    ) -> Result<Vec<Vec<u8>>, Error> {
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

        grovedb_proof.boundaries(path).map_err(|e| {
            Error::Proof(ProofError::CorruptedProof(format!(
                "error extracting boundary keys: {}",
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

    fn insert_test_items(drive: &Drive, count: u8, platform_version: &PlatformVersion) {
        use crate::drive::RootTree;
        use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use crate::util::batch::GroveDbOpBatch;

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
    fn should_return_boundary_key_from_range_after_proof() {
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

        let path_refs: Vec<&[u8]> = path_vecs.iter().map(|v| v.as_slice()).collect();
        let boundaries = Drive::verify_boundaries(&proof, &path_refs, platform_version)
            .expect("should extract boundaries");

        assert!(
            boundaries.contains(&vec![3u8]),
            "key 3 should be a boundary in RangeAfter proof, got: {:?}",
            boundaries
        );
    }

    #[test]
    fn should_return_empty_for_inclusive_range() {
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

        let path_refs: Vec<&[u8]> = path_vecs.iter().map(|v| v.as_slice()).collect();
        let boundaries = Drive::verify_boundaries(&proof, &path_refs, platform_version)
            .expect("should extract boundaries");

        assert!(
            !boundaries.contains(&vec![3u8]),
            "inclusive range start should not be a boundary"
        );
    }

    #[test]
    fn should_reject_invalid_proof_bytes() {
        let platform_version = PlatformVersion::latest();
        let result = Drive::verify_boundaries(&[0xff, 0xfe], &[b"test"], platform_version);
        assert!(result.is_err(), "garbage proof bytes should return error");
    }
}
