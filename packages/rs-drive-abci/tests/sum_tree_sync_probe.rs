//! Minimal reproducer / tripwire for a grovedb state sync limitation at the pinned
//! revision (6c882c3): the replication protocol at this revision does not faithfully
//! restore SumTree subtrees. The chunk transfer copies the source's node hashes, so the restored
//! database reproduces the source ROOT hash — but re-opening the restored sum tree
//! and recomputing its root yields a different hash, i.e. the corruption is latent
//! and `verify_grovedb` detects it.
//!
//! This is why `apply_snapshot_chunk` runs the strict `verify_grovedb` check after
//! committing a state sync session, and why the full two-instance state sync
//! integration test (`run_state_sync_between_two_platforms`) is `#[ignore]`d.
//!
//! WHEN THIS TEST STARTS FAILING because no verification issues are reported, the
//! grovedb pin has been fixed: delete this tripwire and un-ignore the full
//! integration test.

use drive::grovedb::{Element, GroveDb};
use drive::grovedb_path::SubtreePath;
use platform_version::version::PlatformVersion;
use std::collections::VecDeque;

#[test]
fn sum_tree_state_sync_restore_is_latently_corrupt_at_pinned_grovedb() {
    let grove_version = &PlatformVersion::latest().drive.grove_version;
    let source_dir = tempfile::tempdir().unwrap();
    let source = GroveDb::open(source_dir.path()).unwrap();

    let root: SubtreePath<[u8; 0]> = SubtreePath::empty();

    source
        .insert(
            root.clone(),
            b"s",
            Element::empty_sum_tree(),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .unwrap();
    let sum_path: &[&[u8]] = &[b"s"];
    for (key, value) in [(b"a", 5i64), (b"b", 7i64)] {
        source
            .insert(
                sum_path,
                key,
                Element::new_sum_item(value),
                None,
                None,
                grove_version,
            )
            .unwrap()
            .unwrap();
    }
    // A normal tree with an item, for contrast: it restores cleanly.
    source
        .insert(
            root.clone(),
            b"n",
            Element::empty_tree(),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .unwrap();
    let normal_path: &[&[u8]] = &[b"n"];
    source
        .insert(
            normal_path,
            b"k",
            Element::new_item(b"v".to_vec()),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .unwrap();

    let app_hash = source.root_hash(None, grove_version).unwrap().unwrap();

    let target_dir = tempfile::tempdir().unwrap();
    let target = GroveDb::open(target_dir.path()).unwrap();
    let mut session = target
        .start_snapshot_syncing(app_hash, 64, 1, grove_version)
        .unwrap();

    let mut queue: VecDeque<Vec<u8>> = VecDeque::from([app_hash.to_vec()]);
    while let Some(chunk_id) = queue.pop_front() {
        let chunk = source
            .fetch_chunk(&chunk_id, None, 1, grove_version)
            .unwrap();
        let next = session
            .apply_chunk(&chunk_id, &chunk, 1, grove_version)
            .unwrap();
        queue.extend(next);
        if session.is_sync_completed() {
            break;
        }
    }
    assert!(session.is_sync_completed());
    target.commit_session(session, grove_version).unwrap();

    // The copied node hashes reproduce the source root hash exactly...
    let target_root = target.root_hash(None, grove_version).unwrap().unwrap();
    assert_eq!(
        target_root, app_hash,
        "restored root hash must match the source"
    );

    // ...but recomputing the restored sum tree exposes the latent corruption.
    let issues = target
        .verify_grovedb(None, true, false, grove_version)
        .unwrap();
    let paths: Vec<String> = issues
        .keys()
        .map(|path| path.iter().map(hex::encode).collect::<Vec<_>>().join("/"))
        .collect();
    assert_eq!(
        paths,
        vec!["73".to_string()], // hex of b"s", the sum tree
        "expected exactly the sum tree to fail verification — if no issues are \
         reported, grovedb has been fixed: delete this tripwire and un-ignore \
         run_state_sync_between_two_platforms"
    );
}
