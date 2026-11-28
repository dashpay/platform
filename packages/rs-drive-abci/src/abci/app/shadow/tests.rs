//! Tests of shadow mode that need no chain: the start, the Core client of the shadow, and
//! the comparison of stored records.

use super::*;
use crate::config::ShadowConfig;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TestPlatformBuilder;
use dpp::dashcore::consensus::encode::serialize;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{BlockHash, ProTxHash, QuorumHash, Transaction as CoreTransaction, Txid};
use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::dashcore_rpc::Error as CoreError;
use std::sync::atomic::{AtomicUsize, Ordering};

fn refused() -> CoreError {
    CoreError::UnexpectedStructure("asked".to_string())
}

#[test]
fn the_comparison_begins_only_where_the_shadow_holds_what_the_primary_holds() {
    let (one, other) = (Some([1u8; 32]), Some([2u8; 32]));
    // Two databases that hold no block, and two at one height with one app hash.
    assert_eq!(begins_on((0, None), (0, None)), Ok(()));
    assert_eq!(begins_on((7, one), (7, one)), Ok(()));
    // A shadow that is ahead: it skips the heights it holds.
    assert_eq!(begins_on((7, one), (8, other)), Ok(()));
    assert_eq!(begins_on((0, None), (1, one)), Ok(()));

    let behind = begins_on((7, one), (5, one)).expect_err("behind");
    assert!(
        behind.starts_with(
            "the shadow's database is at height 5 and the primary's at 7: the shadow is behind"
        ),
        "{behind}"
    );
    assert!(behind.ends_with(REMEDY));

    let another_state = begins_on((7, one), (7, other)).expect_err("another state");
    assert!(
        another_state.starts_with(&format!(
            "at height 7 the primary's app hash is {} and the shadow's is {}: the two databases \
             do not hold the same state",
            "01".repeat(32),
            "02".repeat(32)
        )),
        "{another_state}"
    );
    let no_block = begins_on((0, one), (0, None)).expect_err("one holds a block");
    assert!(
        no_block.contains("the shadow's is none, for it holds no block"),
        "{no_block}"
    );
}

/// A hook that counts how often the node was ended.
fn counting() -> (Arc<AtomicUsize>, Arc<dyn Fn() + Send + Sync>) {
    let ended = Arc::new(AtomicUsize::new(0));
    let counts = Arc::clone(&ended);
    (
        ended,
        Arc::new(move || {
            counts.fetch_add(1, Ordering::AcqRel);
        }),
    )
}

#[test]
fn no_shadow_is_opened_where_the_configuration_names_no_directory() {
    let platform = TestPlatformBuilder::new().build_with_mock_rpc();
    assert!(platform.config.shadow.shadow_db_path.is_none());
    assert!(open_shadow_platform(&platform.platform)
        .expect("no error")
        .is_none());
    let (_, hook) = counting();
    assert!(
        Shadow::for_application(&platform.platform, None, Arc::clone(&hook))
            .expect("no error")
            .is_none()
    );
    let shadow = open_shadow_platform_at(
        &platform.platform,
        platform.config.db_path.join("shadow"),
        ShadowEngine::Other,
        None,
    )
    .expect("a shadow");
    let refused = Shadow::with_settings(
        &platform.platform,
        Some(&shadow),
        &ShadowConfig::default(),
        hook,
    )
    .expect_err("not turned on");
    assert_eq!(
        refused.to_string(),
        "shadow mode is not turned on: SHADOW_DB_PATH is not set"
    );
}

/// A node in shadow mode: the configuration that turns it on, in `directory`.
fn in_shadow_mode(directory: &std::path::Path) -> crate::config::PlatformConfig {
    crate::config::PlatformConfig {
        shadow: ShadowConfig {
            shadow_db_path: Some(directory.join("shadow")),
            ..Default::default()
        },
        rejections_path: Some(directory.join("rejected")),
        ..Default::default()
    }
}

#[test]
fn the_shadow_is_on_the_other_engine_in_its_own_directory_and_writes_no_rejections() {
    let directory = tempfile::tempdir().expect("a directory");
    let platform = TestPlatformBuilder::new()
        .with_config(in_shadow_mode(directory.path()))
        .build_with_mock_rpc();
    // The process is at another protocol version than the primary when the shadow is
    // opened, as it is where an open published one: after it, it is at the primary's.
    let primarys = platform
        .state
        .load()
        .current_platform_version()
        .expect("a version");
    PlatformVersion::set_current(
        PlatformVersion::get(primarys.protocol_version - 1).expect("a version"),
    );
    let shadow = open_shadow_platform(&platform.platform)
        .expect("opened")
        .expect("a shadow");
    assert_eq!(
        shadow.drive.storage_engine(),
        the_other(platform.drive.storage_engine())
    );
    assert_ne!(
        shadow.drive.storage_engine(),
        platform.drive.storage_engine()
    );
    assert_eq!(shadow.config.db_path, directory.path().join("shadow"));
    assert_eq!(shadow.config.rejections_path, None);
    assert_eq!(shadow.config.shadow, Default::default());
    assert_eq!(
        shadow.state.load().current_protocol_version_in_consensus(),
        platform
            .state
            .load()
            .current_protocol_version_in_consensus()
    );
    assert_eq!(
        PlatformVersion::get_current()
            .expect("a version")
            .protocol_version,
        primarys.protocol_version
    );

    // The shadow of the application: its report is beside the shadow's directory, and it
    // says which engines it is of.
    let (ended, hook) = counting();
    let of_application = Shadow::for_application(&platform.platform, Some(&shadow), hook)
        .expect("started")
        .expect("a shadow of the application");
    assert_eq!(format!("{of_application:?}"), "<Shadow>");
    assert_eq!(
        of_application.report().directory(),
        directory.path().join("shadow-report")
    );
    let counts = of_application.report().counts();
    assert_eq!(
        counts.primary_engine,
        platform.drive.storage_engine().to_string()
    );
    assert_eq!(
        counts.shadow_engine,
        shadow.drive.storage_engine().to_string()
    );
    assert_eq!(
        (counts.height, counts.shadow_height, counts.starts),
        (0, 0, 1)
    );
    assert!(std::ptr::eq(
        of_application.platform().expect("an open shadow"),
        &shadow
    ));
    assert!(of_application.applying().is_some());

    // The end of the node: the hook is called once, and every `FinalizeBlock` after it
    // is answered with the error, which names the height the node ended at.
    let said = of_application.end(12).error;
    assert!(said.contains("diverge at height 12"), "{said}");
    assert!(said.contains("SHADOW_ON_DIVERGENCE=stop"), "{said}");
    assert_eq!(of_application.end(15).error, said);
    assert_eq!(ended.load(Ordering::Acquire), 1);
    let app = crate::abci::app::FullAbciApplication::new(&platform.platform);
    let request = proto::RequestFinalizeBlock {
        height: 13,
        ..Default::default()
    };
    let after = of_application
        .finalize_block(&app, request)
        .expect_err("the node is ending");
    assert_eq!(after.error, said);
}

/// A shadow that stopped in an earlier run is not opened again, whatever state its
/// database is in: the node starts, and counts the blocks it commits as without a shadow.
#[test]
fn a_shadow_that_stopped_in_an_earlier_run_is_not_opened_again() {
    let directory = tempfile::tempdir().expect("a directory");
    let platform = TestPlatformBuilder::new()
        .with_config(in_shadow_mode(directory.path()))
        .build_with_mock_rpc();
    let report = directory.path().join("shadow-report");
    assert_eq!(Report::stopped_in(&report).expect("read"), None);
    assert_eq!(Report::engines_in(&report).expect("read"), None);
    {
        let shadow = open_shadow_platform(&platform.platform)
            .expect("opened")
            .expect("a shadow");
        let (_, hook) = counting();
        let of_application = Shadow::for_application(&platform.platform, Some(&shadow), hook)
            .expect("started")
            .expect("a shadow of the application");
        let app = of_application.applying().expect("applying");
        of_application.stop_the_shadow(app, 1, "a storage failure".to_string());
        assert!(of_application.applying().is_none());
    }
    let stopped = Report::stopped_in(&report).expect("read").expect("stopped");
    assert_eq!(
        (stopped.height, stopped.reason.as_str()),
        (1, "a storage failure")
    );
    // What is in the shadow's directory now does not matter: it is not looked at.
    std::fs::remove_dir_all(directory.path().join("shadow")).expect("removed");
    std::fs::write(directory.path().join("shadow"), b"not a database").expect("written");
    assert!(open_shadow_platform(&platform.platform)
        .expect("no error")
        .is_none());
    let (_, hook) = counting();
    let of_application = Shadow::for_application(&platform.platform, None, hook)
        .expect("started")
        .expect("a shadow of the application");
    assert!(of_application.platform().is_none());
    assert!(of_application.applying().is_none());
    let counts = of_application.report().counts();
    assert_eq!(counts.starts, 2);
    assert_eq!(counts.shadow_stopped, Some(stopped));
    assert_eq!(
        Report::engines_in(&report).expect("read"),
        Some((counts.primary_engine.clone(), counts.shadow_engine.clone()))
    );
    assert_ne!(counts.primary_engine, counts.shadow_engine);
}

/// A stop that a comparison of stored records asked for, and that no block wrote down
/// before the node stopped, is a stop at the next start: the shadow is not opened.
#[test]
fn a_stop_that_was_asked_for_and_not_written_down_is_a_stop_at_the_next_start() {
    let directory = tempfile::tempdir().expect("a directory");
    let platform = TestPlatformBuilder::new()
        .with_config(in_shadow_mode(directory.path()))
        .build_with_mock_rpc();
    {
        let shadow = open_shadow_platform(&platform.platform)
            .expect("opened")
            .expect("a shadow");
        let (_, hook) = counting();
        let of_application = Shadow::for_application(&platform.platform, Some(&shadow), hook)
            .expect("started")
            .expect("a shadow of the application");
        of_application
            .report
            .change(|counts| {
                counts.shadow_stop_pending = Some("the stored records differ".to_string())
            })
            .expect("written");
    }
    assert!(open_shadow_platform(&platform.platform)
        .expect("no error")
        .is_none());
    let (_, hook) = counting();
    let of_application = Shadow::for_application(&platform.platform, None, hook)
        .expect("started")
        .expect("a shadow of the application");
    assert!(of_application.applying().is_none());
    let counts = of_application.report().counts();
    assert_eq!(counts.shadow_stop_pending, None);
    let stopped = counts.shadow_stopped.expect("stopped");
    assert_eq!(stopped.height, 1);
    assert_eq!(
        stopped.reason,
        "the stored records differ; written down at a start, with the shadow's database not \
         opened"
    );
}

/// Counts that are beyond the primary's database at a start: the start writes both
/// heights down.
#[test]
fn heights_the_counts_hold_and_the_database_lost_are_written_down_at_a_start() {
    let directory = tempfile::tempdir().expect("a directory");
    let platform = TestPlatformBuilder::new()
        .with_config(in_shadow_mode(directory.path()))
        .build_with_mock_rpc();
    let shadow = open_shadow_platform(&platform.platform)
        .expect("opened")
        .expect("a shadow");
    {
        let (_, hook) = counting();
        let of_application = Shadow::for_application(&platform.platform, Some(&shadow), hook)
            .expect("started")
            .expect("a shadow of the application");
        assert!(of_application.report().counts().rewinds.is_empty());
        of_application
            .report
            .change(|counts| {
                counts.height = 9;
                counts.shadow_height = 9;
                counts.blocks_compared = 9;
            })
            .expect("written");
    }
    let (_, hook) = counting();
    let of_application = Shadow::for_application(&platform.platform, Some(&shadow), hook)
        .expect("started")
        .expect("a shadow of the application");
    let counts = of_application.report().counts();
    assert_eq!(
        counts.rewinds,
        vec![Rewind {
            counted: 9,
            held: 0
        }]
    );
    assert_eq!((counts.height, counts.shadow_height), (0, 0));
    assert_eq!(counts.blocks_in_all(), 9);
    assert!(counts.blocks_uncounted.is_empty());
}

/// Shadow mode that is on and has no shadow, where none stopped, does not start.
#[test]
fn shadow_mode_without_a_shadow_that_did_not_stop_is_refused() {
    let directory = tempfile::tempdir().expect("a directory");
    let platform = TestPlatformBuilder::new()
        .with_config(in_shadow_mode(directory.path()))
        .build_with_mock_rpc();
    let (_, hook) = counting();
    let refused = Shadow::for_application(&platform.platform, None, hook).expect_err("refused");
    assert_eq!(
        refused.to_string(),
        "shadow mode is on and the shadow's database is not open"
    );
}

#[test]
fn a_shadow_in_the_directory_of_the_nodes_own_database_is_refused() {
    let directory = tempfile::tempdir().expect("a directory");
    let config = crate::config::PlatformConfig {
        db_path: directory.path().join("db"),
        shadow: crate::config::ShadowConfig {
            shadow_db_path: Some(directory.path().join("db")),
            ..Default::default()
        },
        ..Default::default()
    };
    let platform = Platform::open_with_client(
        config.db_path.clone(),
        Some(config),
        MockCoreRPCLike::new(),
        None,
    )
    .expect("a platform");
    let Err(refused) = open_shadow_platform(&platform) else {
        panic!("the same directory");
    };
    assert!(
        refused
            .to_string()
            .starts_with("SHADOW_DB_PATH is the directory of the node's own database, "),
        "{refused}"
    );
}

/// Every question the shadow has for Core goes to the primary's client and comes back as
/// that client answers it. A transaction is not sent: the mock would panic if it were
/// asked to send one.
#[test]
fn the_shadow_asks_core_through_the_primarys_client_and_sends_nothing() {
    let mut core = MockCoreRPCLike::new();
    core.expect_get_block_hash()
        .returning(|height| match height {
            7 => Ok(BlockHash::all_zeros()),
            _ => Err(refused()),
        });
    core.expect_get_block_header().returning(|_| Err(refused()));
    core.expect_get_block_time_from_height()
        .returning(|height| Ok(u64::from(height) * 10));
    core.expect_get_best_chain_lock()
        .returning(|| Err(refused()));
    core.expect_submit_chain_lock()
        .returning(|_| Err(refused()));
    core.expect_get_transaction().returning(|_| Err(refused()));
    core.expect_get_asset_unlock_statuses()
        .returning(|indices, height| match (indices.len(), height) {
            (2, 9) => Ok(vec![]),
            _ => Err(refused()),
        });
    core.expect_get_transaction_extended_info()
        .returning(|_| Err(refused()));
    core.expect_get_optional_transaction_extended_info()
        .returning(|_| Ok(None));
    core.expect_get_fork_info().returning(|name| match name {
        "v24" => Ok(None),
        _ => Err(refused()),
    });
    core.expect_get_block().returning(|_| Err(refused()));
    core.expect_get_block_json().returning(|_| Err(refused()));
    core.expect_get_chain_tips().returning(|| Ok(vec![]));
    core.expect_get_quorum_listextended()
        .returning(|_| Err(refused()));
    core.expect_get_quorum_info()
        .returning(|_, _, _| Err(refused()));
    core.expect_get_protx_diff_with_masternodes()
        .returning(|_, _| Err(refused()));
    core.expect_get_masternode_state()
        .returning(|_, _| Err(refused()));
    core.expect_verify_instant_lock()
        .returning(|_, max_height| Ok(max_height == Some(3)));
    core.expect_verify_chain_lock().returning(|_| Ok(true));
    core.expect_masternode_sync_status()
        .returning(|| Err(refused()));
    core.expect_get_credit_pool_balance()
        .returning(|height| Ok(u64::from(height) + 1));
    core.expect_get_coinbase_payload_version()
        .returning(|height| Ok(Some(height as u16)));

    let shadow = ShadowCore::of(&core);
    assert!(format!("{shadow:?}").starts_with("ShadowCore"));
    assert!(shadow.errors_noted().is_empty());
    let zero = BlockHash::all_zeros();
    let txid = Txid::all_zeros();
    let chain_lock = dpp::dashcore::ChainLock {
        block_height: 1,
        block_hash: zero,
        signature: [0; 96].into(),
    };
    let instant_lock = dpp::dashcore::InstantLock::default();

    assert_eq!(shadow.get_block_hash(7).expect("asked"), zero);
    assert!(shadow.get_block_hash(8).is_err());
    assert!(shadow.get_block_header(&zero).is_err());
    assert_eq!(shadow.get_block_time_from_height(4).expect("asked"), 40);
    assert!(shadow.get_best_chain_lock().is_err());
    assert!(shadow.submit_chain_lock(&chain_lock).is_err());
    assert!(shadow.get_transaction(&txid).is_err());
    assert!(shadow
        .get_asset_unlock_statuses(&[1, 2], 9)
        .expect("asked")
        .is_empty());
    assert!(shadow.get_asset_unlock_statuses(&[1], 9).is_err());
    assert!(shadow.get_transaction_extended_info(&txid).is_err());
    assert!(shadow
        .get_optional_transaction_extended_info(&txid)
        .expect("asked")
        .is_none());
    assert!(shadow.get_fork_info("v24").expect("asked").is_none());
    assert!(shadow.get_fork_info("v25").is_err());
    assert!(shadow.get_block(&zero).is_err());
    assert!(shadow.get_block_json(&zero).is_err());
    assert!(shadow.get_chain_tips().expect("asked").is_empty());
    assert!(shadow.get_quorum_listextended(Some(1)).is_err());
    assert!(shadow
        .get_quorum_info(QuorumType::LlmqTest, &QuorumHash::all_zeros(), None)
        .is_err());
    assert!(shadow.get_protx_diff_with_masternodes(None, 5).is_err());
    assert!(shadow
        .get_masternode_state(&ProTxHash::all_zeros(), 5)
        .is_err());
    assert!(shadow
        .verify_instant_lock(&instant_lock, Some(3))
        .expect("asked"));
    assert!(!shadow
        .verify_instant_lock(&instant_lock, None)
        .expect("asked"));
    assert!(shadow.verify_chain_lock(&chain_lock).expect("asked"));
    assert!(shadow.masternode_sync_status().is_err());
    assert_eq!(shadow.get_credit_pool_balance(41).expect("asked"), 42);
    assert_eq!(
        shadow.get_coinbase_payload_version(3).expect("asked"),
        Some(3)
    );

    // Every error Core answered with is noted with its question, in order, and none of
    // the answers that were no errors. They stay noted until they are cleared, and no
    // more than the first 64 are.
    let errors = shadow.errors_noted();
    assert_eq!(shadow.errors_noted(), errors);
    let questions: Vec<&str> = errors
        .iter()
        .map(|error| error.split(": ").next().expect("a question"))
        .collect();
    assert_eq!(
        questions,
        vec![
            "get_block_hash",
            "get_block_header",
            "get_best_chain_lock",
            "submit_chain_lock",
            "get_transaction",
            "get_asset_unlock_statuses",
            "get_transaction_extended_info",
            "get_fork_info",
            "get_block",
            "get_block_json",
            "get_quorum_listextended",
            "get_quorum_info",
            "get_protx_diff_with_masternodes",
            "get_masternode_state",
            "masternode_sync_status",
        ]
    );
    assert!(errors
        .iter()
        .all(|error| error.len() > error.find(": ").expect("a text") + 2));
    for _ in 0..100 {
        assert!(shadow.get_block_hash(8).is_err());
    }
    assert_eq!(shadow.errors_noted().len(), 64);
    assert_eq!(shadow.errors_noted()[..errors.len()], errors[..]);
    shadow.clear_errors();
    assert!(shadow.errors_noted().is_empty());

    // Not sent: the answer is the id of the transaction, and no id for bytes that hold
    // none.
    let transaction = CoreTransaction {
        version: 3,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: None,
    };
    assert_eq!(
        shadow
            .send_raw_transaction(&serialize(&transaction))
            .expect("answered"),
        transaction.txid()
    );
    assert_eq!(
        shadow.send_raw_transaction(&[0xff]).expect("answered"),
        Txid::all_zeros()
    );
}

#[cfg(feature = "shadow-store-compare")]
mod stores {
    use super::*;
    use drive::config::DriveConfig;
    use drive::drive::Drive;
    use drive::grovedb::element::{BackwardReferences, BidirectionalReference};
    use drive::grovedb::reference_path::ReferencePathType;
    use drive::grovedb::{Element, GroveDb};
    use drive::grovedb_path::SubtreePath;
    use std::path::Path;

    fn config(engine: StorageEngine) -> DriveConfig {
        DriveConfig {
            grovedb_storage_engine: engine,
            ..Default::default()
        }
    }

    fn put(grove: &GroveDb, path: &[&[u8]], key: &[u8], element: Element) {
        let version = &PlatformVersion::latest().drive.grove_version;
        let inserted = match path.is_empty() {
            true => grove.insert(SubtreePath::empty(), key, element, None, None, version),
            false => grove.insert(path, key, element, None, None, version),
        };
        inserted.unwrap().expect("an element");
    }

    /// A database on `engine` with trees of several kinds, three deep, and in the deepest
    /// an item that takes backward references. Where it is `referred_to`, a reference in
    /// another tree points at the item, which enters the reference in the item's list of
    /// referrers.
    fn a_database(directory: &Path, engine: StorageEngine, referred_to: bool) -> GroveDb {
        let grove = Drive::open_grovedb(directory, &config(engine)).expect("a database");
        put(&grove, &[], b"a", Element::empty_tree());
        put(&grove, &[], b"b", Element::empty_sum_tree());
        put(&grove, &[b"a"], b"item", Element::new_item(vec![1, 2, 3]));
        put(&grove, &[b"a"], b"n", Element::empty_tree());
        put(&grove, &[b"a"], b"count", Element::empty_count_tree());
        put(&grove, &[b"a", b"count"], b"x", Element::new_item(vec![4]));
        put(&grove, &[b"a", b"n"], b"deep", Element::empty_tree());
        put(
            &grove,
            &[b"a", b"n", b"deep"],
            b"target",
            Element::ItemWithBackwardsReferences(
                vec![9],
                BackwardReferences::with_max_incoming(4),
                None,
            ),
        );
        put(&grove, &[b"b"], b"sum", Element::new_sum_item(5));
        if referred_to {
            let reference = BidirectionalReference {
                forward_reference_path: ReferencePathType::AbsolutePathReference(vec![
                    b"a".to_vec(),
                    b"n".to_vec(),
                    b"deep".to_vec(),
                    b"target".to_vec(),
                ]),
                cascade_on_update: false,
                max_hop: None,
                backward_references: vec![],
            };
            put(
                &grove,
                &[b"a", b"count"],
                b"refers",
                Element::BidirectionalReference(reference, None),
            );
        }
        grove
    }

    fn root(grove: &GroveDb) -> [u8; 32] {
        grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .unwrap()
            .expect("a root")
    }

    fn compared(primary: &GroveDb, shadow: &GroveDb) -> StoreComparison {
        compare_stores(primary, shadow, 3, PlatformVersion::latest())
    }

    /// The same records on the two engines give the same dumps, subtree by subtree.
    #[test]
    fn the_two_engines_dump_the_same_records_to_the_same_bytes() {
        let directory = tempfile::tempdir().expect("a directory");
        let rocksdb = a_database(&directory.path().join("r"), StorageEngine::RocksDb, false);
        let lignindb = a_database(&directory.path().join("l"), StorageEngine::LigninDb, false);
        assert_eq!(root(&rocksdb), root(&lignindb));
        for (primary, shadow) in [(&rocksdb, &lignindb), (&lignindb, &rocksdb)] {
            let comparison = compared(primary, shadow);
            assert_eq!(comparison.error, None);
            assert_eq!(comparison.differing, 0, "{:?}", comparison.differences);
            assert_eq!(comparison.height, 3);
            // a, b, a/n, a/count, a/n/deep.
            assert_eq!(comparison.subtrees_compared, 5);
            assert_eq!(comparison.subtrees_not_dumped, 0);
            assert_eq!(comparison.root_elements_compared, 2);
            assert!(comparison.bytes > 100, "{}", comparison.bytes);
        }
    }

    /// The list of referrers of an element is in no hash, and no public read returns it. A
    /// database in which a reference points at an item and one in which none does hold
    /// the item's own tree with the same elements, as reads return them, and with other
    /// records: the item's record has the referrer. The comparison finds that tree.
    #[test]
    fn a_referrer_list_that_differs_is_found_where_no_read_shows_it() {
        let directory = tempfile::tempdir().expect("a directory");
        let primary = a_database(&directory.path().join("r"), StorageEngine::RocksDb, false);
        let shadow = a_database(&directory.path().join("l"), StorageEngine::LigninDb, true);
        let version = &PlatformVersion::latest().drive.grove_version;
        let path: [&[u8]; 3] = [b"a", b"n", b"deep"];
        let item = |grove: &GroveDb| {
            grove
                .get_raw(SubtreePath::from(path.as_slice()), b"target", None, version)
                .unwrap()
                .expect("the item")
        };
        assert_eq!(item(&primary), item(&shadow));
        let the_items_tree = "the subtree at /61/6e key 64656570: this=Stream";
        let comparison = compared(&primary, &shadow);
        assert_eq!(comparison.error, None);
        assert!(
            comparison
                .differences
                .iter()
                .any(|difference| difference.starts_with(the_items_tree)),
            "{:?}",
            comparison.differences
        );
        // The same on one engine: the difference is the list's and not an engine's.
        let same_engine = a_database(&directory.path().join("r2"), StorageEngine::RocksDb, true);
        assert!(compared(&primary, &same_engine)
            .differences
            .iter()
            .any(|difference| difference.starts_with(the_items_tree)));
        let equal = compared(&shadow, &same_engine);
        assert_eq!(
            (equal.differing, equal.error),
            (0, None),
            "{:?}",
            equal.differences
        );
    }

    /// An element that one database alone holds, a value that differs, and a tree that one
    /// alone holds are each found; more differences than are kept are still counted.
    #[test]
    fn records_that_differ_are_found_and_counted_beyond_those_that_are_kept() {
        let directory = tempfile::tempdir().expect("a directory");
        let primary = a_database(&directory.path().join("r"), StorageEngine::RocksDb, false);
        let shadow = a_database(&directory.path().join("l"), StorageEngine::LigninDb, false);
        put(&shadow, &[b"b"], b"more", Element::new_sum_item(1));
        let comparison = compared(&primary, &shadow);
        assert_eq!(comparison.error, None);
        // The sum tree's own records, and the root, whose element of it holds its sum.
        assert_eq!(comparison.differing, 2, "{:?}", comparison.differences);
        assert!(comparison.differences[0]
            .starts_with("the elements of the root from the first: this has "));
        assert!(comparison.differences[1].starts_with("the subtree at / key 62: "));

        // A tree the shadow lacks: its dump fails there, which is a difference, and the
        // walk goes on.
        put(
            &primary,
            &[b"a", b"count"],
            b"only-here",
            Element::empty_tree(),
        );
        let comparison = compared(&primary, &shadow);
        assert_eq!(comparison.error, None);
        assert!(
            comparison
                .differences
                .iter()
                .any(|difference| difference.contains("key 6f6e6c792d68657265")
                    && difference.contains("other=Failed(")),
            "{:?}",
            comparison.differences
        );

        // Thirty trees that the primary alone holds.
        for number in 0u8..30 {
            put(
                &primary,
                &[b"a", b"n"],
                &[b't', number],
                Element::empty_tree(),
            );
        }
        let comparison = compared(&primary, &shadow);
        assert!(comparison.differing > 30, "{}", comparison.differing);
        assert_eq!(comparison.differences.len(), 20);
    }

    /// A walk over a checkpoint of a database on each engine, whose shadow's item has a
    /// referrer where `referred_to`, with the latches it sets and the hook's count.
    fn a_walk(
        directory: &Path,
        referred_to: bool,
        on_divergence: OnDivergence,
    ) -> (walk::StoreWalk, Arc<Report>, Arc<Latches>, Arc<AtomicUsize>) {
        let primary = a_database(&directory.join("r"), StorageEngine::RocksDb, false);
        let shadow = a_database(&directory.join("l"), StorageEngine::LigninDb, referred_to);
        let checkpoint = |database: &str| directory.join(database).join(walk::DIRECTORY).join("3");
        for (grove, database) in [(&primary, "r"), (&shadow, "l")] {
            std::fs::create_dir_all(directory.join(database).join(walk::DIRECTORY))
                .expect("a directory");
            grove
                .create_checkpoint(checkpoint(database))
                .expect("a checkpoint");
        }
        let report = Arc::new(
            Report::open(&directory.join("report"), "rocksdb", "lignindb").expect("a report"),
        );
        report
            .change(|counts| counts.store_comparisons.running = true)
            .expect("written");
        let (ended, hook) = counting();
        let latches = Arc::new(Latches::new(false, hook));
        let walk = walk::StoreWalk {
            height: 3,
            primary: (checkpoint("r"), config(StorageEngine::RocksDb)),
            shadow: (checkpoint("l"), config(StorageEngine::LigninDb)),
            platform_version: PlatformVersion::latest(),
            report: Arc::clone(&report),
            latches: Arc::clone(&latches),
            on_divergence,
            gate: None,
        };
        (walk, report, latches, ended)
    }

    fn kept_mark(checkpoint: &Path) -> std::path::PathBuf {
        checkpoint.with_file_name("3.kept")
    }

    #[test]
    fn checkpoints_that_hold_the_same_records_are_compared_and_removed() {
        let directory = tempfile::tempdir().expect("a directory");
        let (walk, report, latches, ended) = a_walk(directory.path(), false, OnDivergence::Stop);
        let checkpoints = [walk.primary.0.clone(), walk.shadow.0.clone()];
        walk.run();
        let comparisons = report.counts().store_comparisons;
        assert_eq!(
            (
                comparisons.made,
                comparisons.failed,
                comparisons.differing,
                comparisons.running
            ),
            (1, 0, 0, false)
        );
        let last = comparisons.last.expect("the comparison");
        assert_eq!(
            (last.height, last.subtrees_compared, last.differing),
            (3, 5, 0)
        );
        for checkpoint in &checkpoints {
            assert!(!checkpoint.exists());
            assert!(!kept_mark(checkpoint).exists());
        }
        assert!(report.counts().divergences.is_empty());
        assert!(latches.applies());
        assert_eq!((latches.ended(), ended.load(Ordering::Acquire)), (None, 0));
    }

    /// The records differ: a divergence that is counted and kept, with the two
    /// checkpoints as its evidence. The shadow applies no block after the one the node is
    /// at, and the node goes on.
    #[test]
    fn checkpoints_that_differ_are_a_divergence_and_stay_as_the_evidence() {
        let directory = tempfile::tempdir().expect("a directory");
        let (walk, report, latches, ended) = a_walk(directory.path(), true, OnDivergence::Continue);
        let checkpoints = [walk.primary.0.clone(), walk.shadow.0.clone()];
        walk.run();
        let counts = report.counts();
        let comparisons = &counts.store_comparisons;
        assert_eq!(
            (
                comparisons.made,
                comparisons.failed,
                comparisons.differing,
                comparisons.running
            ),
            (1, 0, 1, false)
        );
        assert_eq!(counts.divergences.get("store"), Some(&1));
        assert_eq!(counts.divergence_heights, vec![3]);
        for checkpoint in &checkpoints {
            assert!(checkpoint.is_dir());
            assert!(kept_mark(checkpoint).is_file());
        }
        let kept = directory
            .path()
            .join("report")
            .join("divergences")
            .join("3");
        let differs = std::fs::read_to_string(kept.join("differs.txt")).expect("differences");
        assert!(
            differs.starts_with("DIFFERS height=3 field=stored_records this=\""),
            "{differs}"
        );
        assert!(
            differs.contains("the subtree at /61/6e key 64656570"),
            "{differs}"
        );
        let kept_comparison: StoreComparison =
            serde_json::from_slice(&std::fs::read(kept.join("stores.json")).expect("the file"))
                .expect("a comparison");
        assert_eq!(Some(kept_comparison), comparisons.last.clone());

        // The node goes on and the shadow does not: the thread that executes blocks is
        // told why, once.
        assert_eq!((latches.ended(), ended.load(Ordering::Acquire)), (None, 0));
        assert!(!latches.applies());
        let reason = "the stored records of the two databases differ at height 3".to_string();
        // It is written with the comparison, for a node that stops before its next block.
        assert_eq!(counts.shadow_stop_pending, Some(reason.clone()));
        assert_eq!(counts.shadow_stopped, None);
        assert_eq!(latches.at_a_block(), (false, Some(reason)));
        assert_eq!(latches.at_a_block(), (false, None));

        // What a start removes of the comparisons' checkpoints: not the kept ones.
        for (checkpoint, engine) in checkpoints
            .iter()
            .zip([StorageEngine::RocksDb, StorageEngine::LigninDb])
        {
            let database = checkpoint
                .parent()
                .and_then(Path::parent)
                .expect("the database");
            walk::remove_what_a_comparison_left(database, engine);
            assert!(checkpoint.is_dir());
        }
    }

    /// The same where the node is to end: it is ended once, at the height of the
    /// comparison.
    #[test]
    fn checkpoints_that_differ_end_a_node_that_is_to_stop() {
        let directory = tempfile::tempdir().expect("a directory");
        let (walk, report, latches, ended) = a_walk(directory.path(), true, OnDivergence::Stop);
        walk.run();
        assert_eq!(report.counts().divergences.get("store"), Some(&1));
        assert_eq!(
            (latches.ended(), ended.load(Ordering::Acquire)),
            (Some(3), 1)
        );
        // The shadow applies no block either, here or after a start.
        let reason = "the stored records of the two databases differ at height 3".to_string();
        assert_eq!(report.counts().shadow_stop_pending, Some(reason.clone()));
        assert_eq!(latches.at_a_block(), (false, Some(reason)));
    }

    /// A shadow that stopped already is not stopped again by a comparison: nothing is left
    /// pending.
    #[test]
    fn checkpoints_that_differ_leave_a_stop_that_is_written_as_it_is() {
        let directory = tempfile::tempdir().expect("a directory");
        let (walk, report, latches, _) = a_walk(directory.path(), true, OnDivergence::Continue);
        let stopped = Stopped {
            height: 2,
            reason: "before".to_string(),
        };
        report
            .change(|counts| counts.shadow_stopped = Some(stopped.clone()))
            .expect("written");
        latches.stop();
        walk.run();
        let counts = report.counts();
        assert_eq!(counts.divergences.get("store"), Some(&1));
        assert_eq!(counts.shadow_stopped, Some(stopped));
        assert_eq!(counts.shadow_stop_pending, None);
        assert_eq!(latches.at_a_block(), (false, None));
    }

    /// A checkpoint that is not opened: the comparison failed, which is counted and is no
    /// divergence, and what it was to walk is removed.
    #[test]
    fn a_checkpoint_that_is_not_opened_is_a_comparison_that_failed() {
        let directory = tempfile::tempdir().expect("a directory");
        let (mut walk, report, latches, ended) =
            a_walk(directory.path(), false, OnDivergence::Stop);
        let made = [walk.primary.0.clone(), walk.shadow.0.clone()];
        walk.shadow.0 = directory.path().join("no such checkpoint");
        walk.run();
        let comparisons = report.counts().store_comparisons;
        assert_eq!(
            (
                comparisons.made,
                comparisons.failed,
                comparisons.differing,
                comparisons.running
            ),
            (0, 1, 0, false)
        );
        assert!(comparisons.last.expect("the comparison").error.is_some());
        assert!(!made[0].exists());
        assert!(report.counts().divergences.is_empty());
        assert!(latches.applies());
        assert_eq!((latches.ended(), ended.load(Ordering::Acquire)), (None, 0));

        // The checkpoint the walk did not know of any more is what a comparison leaves
        // behind where the node stops in the middle of one: the next start removes it.
        assert!(made[1].is_dir());
        walk::remove_what_a_comparison_left(&directory.path().join("l"), StorageEngine::LigninDb);
        assert!(!made[1].exists());
        // A database with no such directory is left alone.
        walk::remove_what_a_comparison_left(&directory.path().join("none"), StorageEngine::RocksDb);
    }

    /// A layer of more elements than one read takes is read to its end, on both
    /// databases, and a difference beyond the first read is found.
    #[test]
    fn a_layer_that_is_longer_than_one_read_is_walked_to_its_end() {
        let directory = tempfile::tempdir().expect("a directory");
        let primary = a_database(&directory.path().join("r"), StorageEngine::RocksDb, false);
        let shadow = a_database(&directory.path().join("l"), StorageEngine::LigninDb, false);
        for grove in [&primary, &shadow] {
            for number in 0u16..2500 {
                let key = number.to_be_bytes();
                put(grove, &[b"a", b"n"], &key, Element::empty_tree());
            }
            put(
                grove,
                &[b"a", b"n", &2400u16.to_be_bytes()],
                b"x",
                Element::new_item(vec![1]),
            );
        }
        let comparison = compared(&primary, &shadow);
        assert_eq!((comparison.error.clone(), comparison.differing), (None, 0));
        assert_eq!(comparison.subtrees_compared, 5 + 2500);

        // An item in a tree that the first read of its layer does not reach.
        put(
            &shadow,
            &[b"a", b"n", &2450u16.to_be_bytes()],
            b"y",
            Element::new_item(vec![2]),
        );
        let comparison = compared(&primary, &shadow);
        assert_eq!(comparison.error, None);
        assert!(
            comparison
                .differences
                .iter()
                .any(|difference| difference.starts_with("the subtree at /61/6e key 0992: ")),
            "{:?}",
            comparison.differences
        );
    }
}

mod starts {
    use super::*;

    /// The counts as a start finds them after `open` counted it as the `starts`th.
    fn counts(starts: u64, height: u64) -> Counts {
        Counts {
            starts,
            height,
            shadow_height: height,
            blocks_compared: height,
            first_counted_height: (height > 0).then_some(1),
            ..Default::default()
        }
    }

    fn uncounted(counts: &Counts) -> Vec<(u64, u64)> {
        counts
            .blocks_uncounted
            .iter()
            .map(|range| (range.height, range.through))
            .collect()
    }

    /// A first start on databases that hold blocks counts none of them: the node did not
    /// commit them in shadow mode.
    #[test]
    fn a_first_start_counts_nothing_of_what_the_databases_hold() {
        let mut counts = counts(1, 0);
        at_a_start(&mut counts, 500, 501, &None, 1);
        assert_eq!((counts.height, counts.shadow_height), (500, 501));
        assert_eq!(counts.blocks_in_all(), 0);
        assert_eq!(counts.first_counted_height, None);
        assert!(counts.rewinds.is_empty());
    }

    /// Blocks the primary holds and the counts do not are written down as uncounted,
    /// whatever was kept at their heights and whether or not the shadow stopped: nothing
    /// is said of what became of them.
    #[test]
    fn blocks_that_were_not_counted_are_written_down_as_uncounted() {
        let behind = || Counts {
            divergence_heights: vec![3, 12],
            blocks_compared: 9,
            blocks_diverged: 1,
            ..counts(2, 10)
        };
        let mut counts = behind();
        at_a_start(&mut counts, 14, 15, &None, 1);
        assert_eq!(uncounted(&counts), vec![(11, 14)]);
        assert_eq!((counts.blocks_compared, counts.blocks_diverged), (9, 1));
        assert!(counts.blocks_not_compared.is_empty());
        assert_eq!(counts.blocks_without_shadow, 0);
        assert_eq!((counts.height, counts.shadow_height), (14, 15));
        // The heights are in no count of blocks.
        assert_eq!(
            (counts.blocks_in_all(), counts.blocks_uncounted_in_all()),
            (10, 4)
        );
        assert_eq!(counts.shadow_stopped, None);

        // The same where the shadow stopped, with a stop that was pending.
        let stopped = Stopped {
            height: 13,
            reason: "a reason".to_string(),
        };
        let mut counts = behind();
        counts.shadow_stop_pending = Some("asked".to_string());
        at_a_start(&mut counts, 14, 12, &Some(stopped.clone()), 1);
        assert_eq!(uncounted(&counts), vec![(11, 14)]);
        assert_eq!(counts.blocks_without_shadow, 0);
        assert_eq!(counts.blocks_in_all(), 10);
        assert_eq!(counts.shadow_stopped, Some(stopped));
        assert_eq!(counts.shadow_stop_pending, None);

        // Counts that are at the primary's height have nothing to add.
        let mut counts = behind();
        at_a_start(&mut counts, 10, 10, &None, 1);
        assert_eq!(counts.blocks_in_all(), 10);
        assert!(counts.blocks_uncounted.is_empty());
        assert!(counts.rewinds.is_empty());
    }

    /// A chain whose first block is not the first height: the heights before it are no
    /// blocks, and none of them is written down.
    #[test]
    fn heights_below_the_first_of_the_chain_are_no_blocks() {
        let mut counts = counts(2, 0);
        at_a_start(&mut counts, 101, 101, &None, 100);
        assert_eq!(uncounted(&counts), vec![(100, 101)]);
        assert_eq!(
            (counts.blocks_in_all(), counts.blocks_uncounted_in_all()),
            (0, 2)
        );
        // A chain that has not begun has no block to write down.
        let mut counts = self::counts(2, 0);
        at_a_start(&mut counts, 0, 0, &None, 100);
        assert!(counts.blocks_uncounted.is_empty());
        assert!(counts.rewinds.is_empty());
    }

    /// Counts beyond the primary's database: the start is written down with both
    /// heights, and nothing is said of how many blocks will be committed again.
    #[test]
    fn a_start_below_the_counts_is_written_down() {
        let mut counts = counts(2, 20);
        at_a_start(&mut counts, 17, 17, &None, 1);
        assert_eq!(
            counts.rewinds,
            vec![Rewind {
                counted: 20,
                held: 17
            }]
        );
        assert_eq!((counts.height, counts.blocks_in_all()), (17, 20));
        assert!(counts.blocks_uncounted.is_empty());
        // A database that was put back to before the chain's first block.
        let mut counts = self::counts(2, 100);
        at_a_start(&mut counts, 0, 0, &None, 100);
        assert_eq!(
            counts.rewinds,
            vec![Rewind {
                counted: 100,
                held: 0
            }]
        );
        assert!(counts.blocks_uncounted.is_empty());
    }
}

#[cfg(feature = "shadow-store-compare")]
mod what_a_start_empties {
    use super::*;
    use std::path::Path;

    /// The directory of the comparisons' checkpoints is shadow mode's own: a start
    /// removes whatever is in it, a checkpoint that was being made and anything else,
    /// but what is marked as kept and the marks.
    #[test]
    fn everything_but_what_is_kept_is_removed() {
        for engine in [StorageEngine::RocksDb, StorageEngine::LigninDb] {
            let directory = tempfile::tempdir().expect("a directory");
            let own = directory.path().join(walk::DIRECTORY);
            for name in ["7", "8", "9.being-made", "notes"] {
                std::fs::create_dir_all(own.join(name)).expect("a directory");
                std::fs::write(own.join(name).join("part"), b"a part").expect("a file");
            }
            std::fs::write(own.join("8.kept"), b"").expect("a mark");
            std::fs::write(own.join("a file"), b"").expect("a file");
            std::fs::create_dir_all(directory.path().join("beside")).expect("a directory");
            walk::remove_what_a_comparison_left(directory.path(), engine);
            let mut left: Vec<String> = std::fs::read_dir(&own)
                .expect("the directory")
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            left.sort();
            assert_eq!(left, vec!["8", "8.kept"], "{engine}");
            assert!(own.join("8").join("part").is_file());
            assert!(directory.path().join("beside").is_dir());
        }
    }

    /// A symbolic link in the directory is removed as a link, and what it leads to stays.
    /// Where the directory itself is a link, nothing is removed.
    #[cfg(unix)]
    #[test]
    fn a_link_is_not_followed() {
        use std::os::unix::fs::symlink;
        for engine in [StorageEngine::RocksDb, StorageEngine::LigninDb] {
            let directory = tempfile::tempdir().expect("a directory");
            let elsewhere = directory.path().join("elsewhere");
            std::fs::create_dir_all(elsewhere.join("7")).expect("a directory");
            std::fs::write(elsewhere.join("status.json"), b"{}").expect("a file");

            let database = directory.path().join("db");
            let own = database.join(walk::DIRECTORY);
            std::fs::create_dir_all(&own).expect("a directory");
            symlink(&elsewhere, own.join("5")).expect("a link to a directory");
            symlink(elsewhere.join("status.json"), own.join("status")).expect("a link to a file");
            walk::remove_what_a_comparison_left(&database, engine);
            assert_eq!(std::fs::read_dir(&own).expect("the directory").count(), 0);
            assert!(elsewhere.join("7").is_dir());
            assert!(elsewhere.join("status.json").is_file());

            let linked = directory.path().join("linked");
            std::fs::create_dir_all(&linked).expect("a directory");
            symlink(&elsewhere, linked.join(walk::DIRECTORY)).expect("a link");
            walk::remove_what_a_comparison_left(&linked, engine);
            assert!(elsewhere.join("7").is_dir());
            assert!(elsewhere.join("status.json").is_file());
        }
    }

    /// Shadow mode does not start with a directory it or the node writes to inside the
    /// directory a start empties, of either database, however the path gets there: by a
    /// path that is not there yet, or through a symbolic link.
    #[test]
    fn a_directory_that_is_written_to_is_not_inside_it() {
        let directory = tempfile::tempdir().expect("a directory");
        let at = |name: &str| directory.path().join(name);
        let (primary, shadow) = (at("db"), at("shadow"));
        for database in [&primary, &shadow] {
            std::fs::create_dir_all(database.join(walk::DIRECTORY)).expect("a directory");
        }
        let refused = |report: &Path| walk::inside_what_a_start_empties(&primary, &shadow, report);
        assert_eq!(refused(&at("shadow-report")), None);
        assert_eq!(refused(&primary.join("report")), None);
        assert_eq!(refused(&primary.join("shadow-compare-report")), None);

        // Inside, at, and deeper than is there yet.
        let said = refused(&primary.join(walk::DIRECTORY).join("7")).expect("refused");
        assert!(said.starts_with("SHADOW_REPORT_PATH is "), "{said}");
        assert!(
            said.ends_with("is emptied at a start") && said.contains(", which is inside "),
            "{said}"
        );
        assert!(refused(&shadow.join(walk::DIRECTORY)).is_some());
        assert!(refused(&shadow.join(walk::DIRECTORY).join("report").join("deeper")).is_some());

        // Through a symbolic link to the directory, and through one to its database.
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(primary.join(walk::DIRECTORY), at("elsewhere"))
                .expect("a link");
            assert!(refused(&at("elsewhere").join("report")).is_some());
            std::os::unix::fs::symlink(&primary, at("database")).expect("a link");
            assert!(refused(&at("database").join(walk::DIRECTORY).join("report")).is_some());
            assert_eq!(refused(&at("database").join("report")), None);

            // A report that is outside, and that its configured path reaches through a
            // link inside the directory: a start would remove the link.
            std::fs::create_dir_all(at("outside")).expect("a directory");
            let through = shadow.join(walk::DIRECTORY).join("current");
            std::os::unix::fs::symlink(at("outside"), &through).expect("a link");
            let said = refused(&through.join("report")).expect("refused");
            assert!(said.contains(", which leads through "), "{said}");

            // A report whose path is a link to that link, with nothing of the directory
            // in what is written or in where the path ends; the same through a link
            // whose target is written from where the link is.
            std::os::unix::fs::symlink(&through, at("a-link-to-the-link")).expect("a link");
            let said = refused(&at("a-link-to-the-link")).expect("refused");
            assert!(said.contains(", which leads through "), "{said}");
            assert!(refused(&at("a-link-to-the-link").join("report")).is_some());
            let beside = Path::new("shadow").join(walk::DIRECTORY).join("current");
            std::os::unix::fs::symlink(beside, at("a-link-from-here")).expect("a link");
            assert!(refused(&at("a-link-from-here")).is_some());
            std::fs::remove_file(&through).expect("the link removed");

            // Links that lead to each other: the way ends, and nothing of it is inside.
            std::os::unix::fs::symlink(at("there"), at("back")).expect("a link");
            std::os::unix::fs::symlink(at("back"), at("there")).expect("a link");
            assert_eq!(refused(&at("there").join("report")), None);

            // The directory itself a link to the report's: the report is where the
            // directory's name leads.
            let report = at("a-report");
            std::fs::create_dir_all(&report).expect("a directory");
            let third = at("third");
            std::fs::create_dir_all(&third).expect("a directory");
            std::os::unix::fs::symlink(&report, third.join(walk::DIRECTORY)).expect("a link");
            let said =
                walk::inside_what_a_start_empties(&third, &shadow, &report).expect("refused");
            assert!(said.starts_with("SHADOW_REPORT_PATH is "), "{said}");
        }

        // A path that goes through a checkpoint a start removes, and up again: when the
        // checkpoint is gone the path leads nowhere.
        let checkpoint = primary.join(walk::DIRECTORY).join("7");
        std::fs::create_dir_all(&checkpoint).expect("a directory");
        let through = checkpoint.join("..").join("..").join("report");
        assert!(refused(&through).is_some());
        assert_eq!(refused(&primary.join("report")), None);

        // A path that goes up from a directory that is not there.
        let roundabout = at("not-there")
            .join("..")
            .join("db")
            .join(walk::DIRECTORY)
            .join("report");
        assert!(refused(&roundabout).is_some());
        assert_eq!(
            refused(&at("not-there").join("..").join("shadow-report")),
            None
        );

        // A path whose last name is inside and that ends outside leads through.
        let said = refused(&primary.join(walk::DIRECTORY).join("..")).expect("refused");
        assert!(said.contains(", which leads through "), "{said}");

        // The primary's database written with a directory and a `..` after it: what a
        // start empties is in the directory where that path ends.
        std::fs::create_dir_all(primary.join("pad")).expect("a directory");
        let said = walk::inside_what_a_start_empties(
            &primary.join("pad").join(".."),
            &shadow,
            &primary.join(walk::DIRECTORY).join("current").join("report"),
        )
        .expect("refused");
        assert!(said.starts_with("SHADOW_REPORT_PATH is "), "{said}");
        assert_eq!(
            walk::inside_what_a_start_empties(
                &primary.join("pad").join(".."),
                &shadow,
                &primary.join("pad").join(walk::DIRECTORY).join("report"),
            ),
            None
        );

        // The shadow's database inside the primary's, and the primary's inside the
        // shadow's: a start that empties the one would empty the other.
        let report = at("shadow-report");
        let said = walk::inside_what_a_start_empties(
            &primary,
            &primary.join(walk::DIRECTORY).join("shadow"),
            &report,
        )
        .expect("refused");
        assert!(said.starts_with("SHADOW_DB_PATH is "), "{said}");
        let said = walk::inside_what_a_start_empties(
            &shadow.join(walk::DIRECTORY).join("db"),
            &shadow,
            &report,
        )
        .expect("refused");
        assert!(said.starts_with("DB_PATH is "), "{said}");
    }
}
