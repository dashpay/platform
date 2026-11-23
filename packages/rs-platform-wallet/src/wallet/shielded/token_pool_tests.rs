use std::sync::Mutex;

use dashcore::Network;
use grovedb_commitment_tree::{
    DashMemo, Domain, ExtractedNoteCommitment, Note, NoteValue, Nullifier, OrchardDomain,
    RandomSeed, Rho, ValueCommitTrapdoor, ValueCommitment,
};
use rand::{rngs::OsRng, RngCore};

use super::*;
use crate::wallet::shielded::file_store::FileBackedShieldedStore;
use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::store::ShieldedStore;

const WALLET: WalletId = [0x11; 32];
const OTHER_WALLET: WalletId = [0x22; 32];

/// An in-memory connection with the token pool tables.
struct TestTables(Mutex<Connection>);

impl TestTables {
    fn new() -> RwLock<Self> {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        create_tables(&conn).expect("create tables");
        RwLock::new(Self(Mutex::new(conn)))
    }
}

impl TokenPoolConnection for TestTables {
    fn with_token_pool_conn<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, PlatformWalletError>,
    ) -> Result<T, PlatformWalletError> {
        f(&mut self.0.lock().expect("test connection"))
    }
}

/// Something to run while a chunk read is on the network.
type ReadHook = Box<dyn FnOnce() + Send>;

/// A token pool served from memory, one chunk per request.
struct TestPool {
    notes: Mutex<Vec<ShieldedEncryptedNote>>,
    capacity: u32,
    /// Notes the pool claims at the next requests from a start: the answer
    /// a node one block behind gives, as many as the slice says.
    behind: Mutex<BTreeMap<u64, usize>>,
    failing: Mutex<bool>,
    requests: Mutex<Vec<u64>>,
    /// Run once when the chunk at this start is read, before it is
    /// answered: a purge landing while the scan is on the network.
    on_read: Mutex<Option<(u64, ReadHook)>>,
}

impl TestPool {
    fn new(notes: Vec<ShieldedEncryptedNote>) -> Self {
        Self {
            notes: Mutex::new(notes),
            capacity: CHUNK_SIZE as u32,
            behind: Mutex::new(BTreeMap::new()),
            failing: Mutex::new(false),
            requests: Mutex::new(Vec::new()),
            on_read: Mutex::new(None),
        }
    }

    fn push(&self, note: ShieldedEncryptedNote) {
        self.notes.lock().unwrap().push(note);
    }

    fn requests(&self) -> Vec<u64> {
        std::mem::take(&mut *self.requests.lock().unwrap())
    }
}

#[async_trait]
impl TokenPoolNoteSource for TestPool {
    async fn token_pool_chunk(
        &self,
        _token_id: Identifier,
        start_index: u64,
        count: u32,
    ) -> Result<TokenPoolChunk, PlatformWalletError> {
        assert_eq!(start_index % CHUNK_SIZE, 0, "start must be chunk-aligned");
        self.requests.lock().unwrap().push(start_index);
        let hook = {
            let mut slot = self.on_read.lock().unwrap();
            match slot.take() {
                Some((at, hook)) if at == start_index => Some(hook),
                other => {
                    *slot = other;
                    None
                }
            }
        };
        if let Some(hook) = hook {
            hook();
        }
        if *self.failing.lock().unwrap() {
            return Err(PlatformWalletError::ShieldedSyncFailed(
                "node unreachable".to_string(),
            ));
        }
        let notes = self.notes.lock().unwrap();
        let start = start_index as usize;
        let mut end = (start + count as usize).min(notes.len());
        if let Some(short) = self.behind.lock().unwrap().remove(&start_index) {
            end = end.min(start + short);
        }
        Ok(TokenPoolChunk {
            notes: notes.get(start..end).map(<[_]>::to_vec).unwrap_or_default(),
            total_count: notes.len() as u64,
            height: 100 + notes.len() as u64,
            protocol_version: 15,
        })
    }

    fn token_pool_chunk_capacity(&self) -> u32 {
        self.capacity
    }
}

fn token() -> ShieldedTokenPoolWatch {
    ShieldedTokenPoolWatch {
        token_id: Identifier::from([0x77; 32]),
        data_contract_id: Identifier::from([0x66; 32]),
        token_contract_position: 0,
    }
}

fn views(seed: u8, account: u32) -> AccountViewingKeys {
    OrchardKeySet::from_seed(&[seed; 64], Network::Testnet, account)
        .expect("derive keys")
        .viewing_keys()
}

fn random_rho() -> ([u8; 32], Rho) {
    let mut rng = OsRng;
    loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let (Some(_), Some(rho)) = (
            Nullifier::from_bytes(&b).into_option(),
            Rho::from_bytes(&b).into_option(),
        ) {
            return (b, rho);
        }
    }
}

/// A pool entry paying `value` to `to`'s default address. Its action spends
/// `spends` when given (the entry reveals that nullifier), a random one
/// otherwise. Returns the entry and the nullifier that spends the new note.
fn note_to(
    to: &AccountViewingKeys,
    value: u64,
    spends: Option<[u8; 32]>,
) -> (ShieldedEncryptedNote, [u8; 32]) {
    let mut rng = OsRng;
    let (nf, rho) = match spends {
        Some(nf) => (nf, Rho::from_bytes(&nf).into_option().expect("rho")),
        None => random_rho(),
    };
    let rseed = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rseed) = RandomSeed::from_bytes(b, &rho).into_option() {
            break rseed;
        }
    };
    let note_value = NoteValue::from_raw(value);
    let note = Note::from_parts(to.default_address, note_value, rho, rseed)
        .into_option()
        .expect("valid note");
    let cmx = ExtractedNoteCommitment::from(note.commitment());
    let rcv = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rcv) = ValueCommitTrapdoor::from_bytes(b).into_option() {
            break rcv;
        }
    };
    let cv = ValueCommitment::derive(note_value - NoteValue::from_raw(0), rcv);
    let encryption = grovedb_commitment_tree::OrchardNoteEncryption::<DashMemo>::new(
        Some(to.outgoing_viewing_key.clone()),
        note,
        [0; 36],
    );
    let epk = OrchardDomain::<DashMemo>::epk_bytes(encryption.epk());
    let mut encrypted_note = epk.0.to_vec();
    encrypted_note.extend_from_slice(encryption.encrypt_note_plaintext().as_ref());
    encrypted_note.extend_from_slice(&encryption.encrypt_outgoing_plaintext(&cv, &cmx, &mut rng));
    (
        ShieldedEncryptedNote {
            cmx: cmx.to_bytes().to_vec(),
            nullifier: nf.to_vec(),
            cv_net: cv.to_bytes().to_vec(),
            encrypted_note,
        },
        note.nullifier(&to.full_viewing_key).to_bytes(),
    )
}

/// An entry no wallet here decrypts, revealing a random nullifier.
fn foreign_entry() -> ShieldedEncryptedNote {
    let (nf, _) = random_rho();
    let mut cmx = [0u8; 32];
    OsRng.fill_bytes(&mut cmx);
    // Below 2^254, so a valid Pallas base field element, as a commitment is.
    cmx[31] &= 0x3F;
    ShieldedEncryptedNote {
        cmx: cmx.to_vec(),
        nullifier: nf.to_vec(),
        cv_net: vec![0; 32],
        encrypted_note: vec![0; 216],
    }
}

async fn watch(tables: &RwLock<TestTables>, wallet_id: WalletId) {
    tables
        .read()
        .await
        .with_token_pool_conn(|conn| insert_watch(conn, wallet_id, &token()))
        .expect("watch");
}

async fn balance_rows(
    tables: &RwLock<TestTables>,
    wallet_id: WalletId,
    accounts: &[u32],
) -> Vec<ShieldedTokenPoolBalance> {
    tables
        .read()
        .await
        .with_token_pool_conn(|conn| balances(conn, wallet_id, accounts))
        .expect("balances")
}

#[tokio::test]
async fn should_find_a_wallet_s_token_notes_and_drop_the_ones_the_pool_shows_spent() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let mine = views(1, 0);
    let (first, first_nf) = note_to(&mine, 40, None);
    let (second, _) = note_to(&mine, 2, None);
    let pool = TestPool::new(vec![foreign_entry(), first, second]);
    let accounts = vec![(SubwalletId::new(WALLET, 0), mine.clone())];

    let summary = sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    assert_eq!(summary.new_notes, 2);
    assert!(summary.completed.contains(&token().token_id));
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.unspent_note_count), (42, 2));
    assert_eq!(row.scanned_note_count, Some(3));
    assert_eq!(row.pool_note_count, Some(3));

    // A later action spends the first note: its nullifier shows up in the
    // pool, and the change it pays back is found.
    let (change, _) = note_to(&mine, 30, Some(first_nf));
    pool.push(change);
    let summary = sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    assert_eq!((summary.new_notes, summary.newly_spent), (1, 1));
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.unspent_note_count), (32, 2));
    assert_eq!(row.scanned_note_count, Some(4));
}

#[tokio::test]
async fn should_end_spent_a_note_received_and_spent_within_one_chunk() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let mine = views(1, 0);
    let (received, nf) = note_to(&mine, 9, None);
    let (_, _) = note_to(&mine, 1, None);
    let spend = ShieldedEncryptedNote {
        nullifier: nf.to_vec(),
        ..foreign_entry()
    };
    let pool = TestPool::new(vec![received, spend]);
    let accounts = vec![(SubwalletId::new(WALLET, 0), mine)];

    let summary = sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    assert_eq!((summary.new_notes, summary.newly_spent), (1, 1));
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.unspent_note_count), (0, 0));
}

/// The gap #6109 is about: a chunk answered a block behind comes back short
/// while the pool already holds notes past it. The scan must not read past
/// the short chunk, and the next pass must pick up every note it skipped.
#[tokio::test]
async fn should_not_scan_past_a_short_chunk_and_recover_its_notes_next_pass() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let mine = views(1, 0);
    let mut entries: Vec<ShieldedEncryptedNote> = (0..CHUNK_SIZE as usize + 2)
        .map(|_| foreign_entry())
        .collect();
    let (in_first_chunk, _) = note_to(&mine, 5, None);
    let (in_second_chunk, _) = note_to(&mine, 7, None);
    entries[10] = in_first_chunk;
    entries[CHUNK_SIZE as usize + 1] = in_second_chunk;
    let pool = TestPool::new(entries);
    pool.behind.lock().unwrap().insert(0, 5);
    let accounts = vec![(SubwalletId::new(WALLET, 0), mine)];

    sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    assert_eq!(
        pool.requests(),
        vec![0],
        "the short chunk ends the pass: the next chunk is never read"
    );
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!(row.scanned_note_count, Some(5));
    assert_eq!(row.balance, 0);

    sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    assert_eq!(pool.requests(), vec![0, CHUNK_SIZE]);
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!(row.scanned_note_count, Some(CHUNK_SIZE + 2));
    assert_eq!((row.balance, row.unspent_note_count), (12, 2));
}

#[tokio::test]
async fn should_scan_an_account_added_later_from_the_start_without_counting_notes_twice() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let first = views(1, 0);
    let second = views(1, 1);
    let (to_first, _) = note_to(&first, 3, None);
    let (to_second, _) = note_to(&second, 4, None);
    let pool = TestPool::new(vec![to_first, to_second]);

    let only_first = vec![(SubwalletId::new(WALLET, 0), first.clone())];
    sync_token_pools(&pool, &tables, &only_first, &CancellationToken::new()).await;
    let both = vec![
        (SubwalletId::new(WALLET, 0), first),
        (SubwalletId::new(WALLET, 1), second),
    ];
    let summary = sync_token_pools(&pool, &tables, &both, &CancellationToken::new()).await;
    assert_eq!(summary.new_notes, 1, "only the new account's note is new");
    let rows = balance_rows(&tables, WALLET, &[0, 1]).await;
    assert_eq!(rows[0].balance, 3);
    assert_eq!(rows[1].balance, 4);
}

#[tokio::test]
async fn should_scan_only_the_wallets_that_watch_the_pool() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let watching = views(1, 0);
    let other = views(2, 0);
    let (to_watching, _) = note_to(&watching, 3, None);
    let (to_other, _) = note_to(&other, 4, None);
    let pool = TestPool::new(vec![to_watching, to_other]);
    let accounts = vec![
        (SubwalletId::new(WALLET, 0), watching),
        (SubwalletId::new(OTHER_WALLET, 0), other),
    ];

    let summary = sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    assert_eq!(summary.new_notes, 1);
    assert!(balance_rows(&tables, OTHER_WALLET, &[0]).await.is_empty());
}

#[tokio::test]
async fn should_keep_a_failed_scan_s_error_until_a_read_succeeds() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let mine = views(1, 0);
    let (note, _) = note_to(&mine, 6, None);
    let pool = TestPool::new(vec![note]);
    *pool.failing.lock().unwrap() = true;
    let accounts = vec![(SubwalletId::new(WALLET, 0), mine)];

    let summary = sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    assert!(summary.failed.contains_key(&token().token_id));
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert!(row
        .last_error
        .as_deref()
        .is_some_and(|e| e.contains("node unreachable")));
    assert_eq!(row.scanned_note_count, None, "never scanned");

    *pool.failing.lock().unwrap() = false;
    sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.last_error.clone()), (6, None));
}

#[tokio::test]
async fn should_read_nothing_once_cancelled() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let pool = TestPool::new(vec![foreign_entry()]);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let summary = sync_token_pools(
        &pool,
        &tables,
        &[(SubwalletId::new(WALLET, 0), views(1, 0))],
        &cancel,
    )
    .await;
    assert!(pool.requests().is_empty());
    assert!(summary.completed.is_empty());
}

#[tokio::test]
async fn should_forget_a_wallet_s_notes_and_an_unwatched_pool_s_leaves_on_unwatch() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    watch(&tables, OTHER_WALLET).await;
    let mine = views(1, 0);
    let (note, _) = note_to(&mine, 6, None);
    let pool = TestPool::new(vec![note]);
    let accounts = vec![(SubwalletId::new(WALLET, 0), mine)];
    sync_token_pools(&pool, &tables, &accounts, &CancellationToken::new()).await;

    let leaf_count = |conn: &mut Connection| {
        conn.query_row(
            "SELECT COUNT(*) FROM shielded_token_pool_leaves",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(storage("count"))
    };
    let store = tables.read().await;
    store
        .with_token_pool_conn(|conn| remove_watch(conn, WALLET, token().token_id))
        .expect("unwatch");
    assert!(store
        .with_token_pool_conn(|conn| notes(conn, SubwalletId::new(WALLET, 0), token().token_id))
        .expect("notes")
        .is_empty());
    assert_eq!(
        store.with_token_pool_conn(leaf_count).expect("count"),
        1,
        "another wallet still watches the pool"
    );
    store
        .with_token_pool_conn(|conn| remove_watch(conn, OTHER_WALLET, token().token_id))
        .expect("unwatch");
    assert_eq!(store.with_token_pool_conn(leaf_count).expect("count"), 0);
}

/// The file store keeps token pools across a reopen, drops a removed
/// wallet's, and drops all of them on a Clear.
#[tokio::test]
async fn should_keep_token_pools_in_the_store_file_until_purged() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("tree.sqlite");
    let mine = views(1, 0);
    let (note, _) = note_to(&mine, 6, None);
    let pool = TestPool::new(vec![note]);
    let accounts = vec![(SubwalletId::new(WALLET, 0), mine)];
    {
        let store = RwLock::new(FileBackedShieldedStore::open_path(&path, 100).expect("open"));
        store
            .read()
            .await
            .with_token_pool_conn(|conn| insert_watch(conn, WALLET, &token()))
            .expect("watch");
        sync_token_pools(&pool, &store, &accounts, &CancellationToken::new()).await;
    }
    let store = RwLock::new(FileBackedShieldedStore::open_path(&path, 100).expect("reopen"));
    let rows = store
        .read()
        .await
        .with_token_pool_conn(|conn| balances(conn, WALLET, &[0]))
        .expect("balances");
    assert_eq!(rows[0].balance, 6, "kept across a reopen");

    store
        .write()
        .await
        .purge_subwallet(SubwalletId::new(WALLET, 0))
        .expect("purge account");
    let rows = store
        .read()
        .await
        .with_token_pool_conn(|conn| balances(conn, WALLET, &[0]))
        .expect("balances");
    assert_eq!(
        (rows.len(), rows[0].balance, rows[0].scanned_note_count),
        (1, 0, None),
        "the wallet still watches the pool, the account starts over"
    );

    store.write().await.purge_all_subwallets().expect("clear");
    let watched = store
        .read()
        .await
        .with_token_pool_conn(|conn| watches(conn))
        .expect("watches");
    assert_eq!(
        watched.get(&WALLET).map(Vec::len),
        Some(1),
        "a Clear keeps the watches, so the next sync scans the pool again"
    );
    sync_token_pools(&pool, &store, &accounts, &CancellationToken::new()).await;
    let rows = store
        .read()
        .await
        .with_token_pool_conn(|conn| balances(conn, WALLET, &[0]))
        .expect("balances");
    assert_eq!(rows[0].balance, 6, "found again after the Clear");
}

/// An unwatch that lands while a chunk is on the network is not undone by
/// that chunk: nothing is written for the unwatched pool, and a later watch
/// scans it from the start and finds every note.
#[tokio::test]
async fn should_not_undo_an_unwatch_that_lands_mid_scan() {
    let tables = std::sync::Arc::new(TestTables::new());
    watch(&tables, WALLET).await;
    let mine = views(1, 0);
    let mut entries: Vec<ShieldedEncryptedNote> = (0..CHUNK_SIZE as usize + 2)
        .map(|_| foreign_entry())
        .collect();
    let (early, _) = note_to(&mine, 5, None);
    let (late, _) = note_to(&mine, 7, None);
    entries[3] = early;
    entries[CHUNK_SIZE as usize + 1] = late;
    let pool = TestPool::new(entries);
    let purging = std::sync::Arc::clone(&tables);
    *pool.on_read.lock().unwrap() = Some((
        CHUNK_SIZE,
        Box::new(move || {
            purging
                .try_read()
                .expect("no writer")
                .with_token_pool_conn(|conn| remove_watch(conn, WALLET, token().token_id))
                .expect("unwatch");
        }),
    ));
    let id = SubwalletId::new(WALLET, 0);
    let accounts = vec![(id, mine)];

    sync_token_pools(&pool, &*tables, &accounts, &CancellationToken::new()).await;
    let store = tables.read().await;
    let (notes_left, watermark_left, leaves_left) = store
        .with_token_pool_conn(|conn| {
            let count = |table: &str| {
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(storage("count"))
            };
            Ok((
                count("shielded_token_pool_notes")?,
                count("shielded_token_pool_watermarks")?,
                count("shielded_token_pool_leaves")?,
            ))
        })
        .expect("counts");
    assert_eq!(
        (notes_left, watermark_left, leaves_left),
        (0, 0, 0),
        "the chunk read across the unwatch writes nothing"
    );
    drop(store);

    watch(&tables, WALLET).await;
    sync_token_pools(&pool, &*tables, &accounts, &CancellationToken::new()).await;
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.unspent_note_count), (12, 2));
    assert_eq!(row.scanned_note_count, Some(CHUNK_SIZE + 2));
}

fn owned(id: SubwalletId, position: u64, value: u64) -> TokenPoolNote {
    TokenPoolNote {
        id,
        token_id: token().token_id,
        position,
        cmx: [position as u8 + 1; 32],
        nullifier: [0xA0 ^ position as u8; 32],
        value,
        note_data: vec![0; 115],
        block_height: 1,
        is_spent: false,
        reserved_by: None,
    }
}

/// Record that `token`'s pool was scanned up to `height`, as a completed scan
/// would.
fn record_scan(conn: &Connection, token_id: Identifier, height: i64) {
    conn.execute(
        "INSERT INTO shielded_token_pool_scans (token_id, pool_note_count, scanned_height) \
         VALUES (?1, 0, ?2) ON CONFLICT(token_id) DO UPDATE SET scanned_height = ?2",
        params![token_id.as_slice(), height],
    )
    .expect("scan row");
}

/// The sweep as the coordinator runs it: find the orphaned reservations, then
/// release them.
fn sweep_all(
    conn: &Connection,
    owners: &BTreeSet<SubwalletId>,
    scanned: &BTreeSet<Identifier>,
    scanned_from: u64,
    live: &BTreeSet<[u8; 32]>,
) -> Result<BTreeSet<(SubwalletId, [u8; 32])>, PlatformWalletError> {
    let scanned_from = raise_sweep_height(conn, scanned_from)?;
    let orphaned = orphaned_reservations(conn, owners, scanned, scanned_from, live)?;
    release_reservations(conn, &orphaned)?;
    Ok(orphaned
        .into_iter()
        .map(|orphan| (orphan.id, orphan.marker))
        .collect())
}

async fn tables_with(notes: &[TokenPoolNote]) -> RwLock<TestTables> {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    for note in notes {
        tables
            .read()
            .await
            .with_token_pool_conn(|conn| insert_note_for_test(conn, note))
            .expect("insert");
    }
    tables
}

#[tokio::test]
async fn should_reserve_the_largest_token_notes_and_leave_them_out_of_the_balance() {
    let id = SubwalletId::new(WALLET, 0);
    let tables = tables_with(&[owned(id, 0, 5), owned(id, 1, 50), owned(id, 2, 20)]).await;

    let (selected, reservation) =
        reserve_token_notes(&*tables.read().await, id, token().token_id, 60, 3).expect("reserve");
    assert_eq!(
        selected.iter().map(|note| note.value).collect::<Vec<_>>(),
        vec![50, 20],
        "largest first, as few as cover the amount"
    );
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!(
        (row.balance, row.unspent_note_count, row.reserved_balance),
        (5, 1, 70)
    );

    reservation.release(&*tables.read().await);
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.reserved_balance), (75, 0));
}

#[tokio::test]
async fn should_refuse_a_token_spend_its_notes_cannot_cover_or_fit_without_reserving() {
    let id = SubwalletId::new(WALLET, 0);
    let tables = tables_with(&[owned(id, 0, 5), owned(id, 1, 6)]).await;

    let err = reserve_token_notes(&*tables.read().await, id, token().token_id, 12, 3)
        .expect_err("11 does not cover 12");
    assert!(
        matches!(
            err,
            PlatformWalletError::TokenShieldedInsufficientBalance {
                available: 11,
                required: 12,
                ..
            }
        ),
        "got {err:?}"
    );
    let err = reserve_token_notes(&*tables.read().await, id, token().token_id, 11, 1)
        .expect_err("two notes are needed, one fits");
    assert!(
        matches!(
            err,
            PlatformWalletError::ShieldedTooManyNotes {
                required_notes: 2,
                max_notes: 1
            }
        ),
        "got {err:?}"
    );
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.reserved_balance), (11, 0));
}

/// The sweep releases a reservation neither a spend in flight nor a saved
/// record holds, only in a pool scanned this pass; a rekeyed reservation
/// whose record is live stays.
#[tokio::test]
async fn should_release_only_orphaned_token_reservations_in_scanned_pools() {
    let id = SubwalletId::new(WALLET, 0);
    let orphan = TokenPoolNote {
        reserved_by: Some([0xD1; 32]),
        ..owned(id, 0, 5)
    };
    let tables = tables_with(&[orphan, owned(id, 1, 7), owned(id, 2, 9)]).await;
    tables
        .read()
        .await
        .with_token_pool_conn(|conn| {
            record_scan(conn, token().token_id, 50);
            Ok(())
        })
        .expect("scan");
    let (_, in_flight_reservation) =
        reserve_token_notes(&*tables.read().await, id, token().token_id, 9, 1).expect("reserve");
    let (_, mut recorded) =
        reserve_token_notes(&*tables.read().await, id, token().token_id, 7, 1).expect("reserve");
    let record = [0xEC; 32];
    recorded
        .rekey(&*tables.read().await, record, true)
        .expect("rekey");
    drop(recorded);

    let tables_ref = &tables;
    let sweep = |scanned: BTreeSet<Identifier>, live: BTreeSet<[u8; 32]>| async move {
        tables_ref
            .read()
            .await
            .with_token_pool_conn(|conn| {
                sweep_all(conn, &BTreeSet::from([id]), &scanned, 50, &live)
            })
            .expect("sweep")
    };
    assert!(
        sweep(BTreeSet::new(), BTreeSet::from([record]))
            .await
            .is_empty(),
        "a pool not scanned this pass keeps its reservations"
    );
    assert_eq!(
        sweep(BTreeSet::from([token().token_id]), BTreeSet::from([record])).await,
        BTreeSet::from([(id, [0xD1; 32])]),
        "only the orphan comes back"
    );
    let row = &balance_rows(&tables, WALLET, &[0]).await[0];
    assert_eq!((row.balance, row.reserved_balance), (5, 16));

    drop(in_flight_reservation);
    let found = tables
        .read()
        .await
        .with_token_pool_conn(|conn| {
            mark_fee_anchor_pruned(conn, id, &record)?;
            orphaned_reservations(
                conn,
                &BTreeSet::from([id]),
                &BTreeSet::from([token().token_id]),
                50,
                &BTreeSet::new(),
            )
        })
        .expect("read");
    assert_eq!(found.len(), 2);
    assert!(
        found
            .iter()
            .all(|orphan| orphan.fee_anchor_pruned == (orphan.marker == record)),
        "only the hold whose fee anchor was marked pruned says so; a rekey alone does not"
    );
    assert_eq!(
        sweep(BTreeSet::from([token().token_id]), BTreeSet::new())
            .await
            .len(),
        2,
        "with its spend gone and no record, every reservation comes back"
    );
}

/// The re-key to a spend's record keeps whether the spend was signed at a
/// version that binds its token bundle to its fee bundle, as the sweep reads
/// it back.
#[tokio::test]
async fn should_keep_the_signing_era_a_rekey_records() {
    let id = SubwalletId::new(WALLET, 0);
    let tables = tables_with(&[owned(id, 0, 5)]).await;
    tables
        .read()
        .await
        .with_token_pool_conn(|conn| {
            record_scan(conn, token().token_id, 50);
            Ok(())
        })
        .expect("scan");
    for (record, signed_bound) in [([0xE1; 32], true), ([0xE2; 32], false)] {
        let (_, mut reservation) =
            reserve_token_notes(&*tables.read().await, id, token().token_id, 5, 1)
                .expect("reserve");
        reservation
            .rekey(&*tables.read().await, record, signed_bound)
            .expect("rekey");
        drop(reservation);
        let found = tables
            .read()
            .await
            .with_token_pool_conn(|conn| {
                orphaned_reservations(
                    conn,
                    &BTreeSet::from([id]),
                    &BTreeSet::from([token().token_id]),
                    50,
                    &BTreeSet::new(),
                )
            })
            .expect("read");
        assert_eq!(
            found
                .iter()
                .map(|orphan| (orphan.marker, orphan.signed_bound))
                .collect::<Vec<_>>(),
            vec![(record, signed_bound)]
        );
        tables
            .read()
            .await
            .with_token_pool_conn(|conn| release_reservations(conn, &found))
            .expect("release");
    }
}

/// A reservation whose notes the scan has seen spent is still judged: at
/// protocol 15 a competing spend of those notes means its own spend can
/// never execute, so its record must still be settled.
#[tokio::test]
async fn should_report_a_reservation_whose_notes_were_spent() {
    let id = SubwalletId::new(WALLET, 0);
    let spent = TokenPoolNote {
        reserved_by: Some([0xD4; 32]),
        is_spent: true,
        ..owned(id, 0, 5)
    };
    let tables = tables_with(&[spent]).await;
    let found = tables
        .read()
        .await
        .with_token_pool_conn(|conn| {
            record_scan(conn, token().token_id, 50);
            orphaned_reservations(
                conn,
                &BTreeSet::from([id]),
                &BTreeSet::from([token().token_id]),
                50,
                &BTreeSet::new(),
            )
        })
        .expect("read");
    assert_eq!(
        found
            .into_iter()
            .map(|orphan| orphan.marker)
            .collect::<Vec<_>>(),
        vec![[0xD4; 32]]
    );
}

/// The Merkle paths a spend gets lead from each note to the root of the
/// leaves kept, which is the anchor returned; a note past them has none.
#[tokio::test]
async fn should_give_token_notes_paths_to_the_root_of_the_kept_leaves() {
    let tables = TestTables::new();
    watch(&tables, WALLET).await;
    let mine = views(1, 0);
    let (first, _) = note_to(&mine, 4, None);
    let (second, _) = note_to(&mine, 6, None);
    let pool = TestPool::new(vec![foreign_entry(), first, foreign_entry(), second]);
    let id = SubwalletId::new(WALLET, 0);
    sync_token_pools(&pool, &tables, &[(id, mine)], &CancellationToken::new()).await;

    let store = tables.read().await;
    let owned_notes = store
        .with_token_pool_conn(|conn| notes(conn, id, token().token_id))
        .expect("notes");
    let (paths, anchor) = store
        .with_token_pool_conn(|conn| spend_paths(conn, token().token_id, &owned_notes))
        .expect("paths");
    assert_eq!(owned_notes.len(), 2, "the scan found both owned notes");
    assert_eq!(
        paths.len(),
        owned_notes.len(),
        "every owned note has a path"
    );
    for (note, (position, path)) in owned_notes.iter().zip(&paths) {
        assert_eq!(*position, note.position);
        let cmx = grovedb_commitment_tree::ExtractedNoteCommitment::from_bytes(&note.cmx)
            .into_option()
            .expect("cmx");
        assert_eq!(path.root(cmx).to_bytes(), anchor.to_bytes());
    }

    let beyond = TokenPoolNote {
        position: 9,
        ..owned_notes[0].clone()
    };
    let err = store
        .with_token_pool_conn(|conn| spend_paths(conn, token().token_id, &[beyond]))
        .expect_err("position 9 is past the kept leaves");
    assert!(matches!(
        err,
        PlatformWalletError::ShieldedMerkleWitnessUnavailable(_)
    ));
}

/// The sweep judges only the accounts it was given: a reservation of an
/// account not registered now (its records unread) is kept, even in a pool
/// another account's scan completed.
#[tokio::test]
async fn should_keep_the_reservations_of_accounts_outside_the_sweep() {
    let scanned_account = SubwalletId::new(WALLET, 0);
    let absent_account = SubwalletId::new(OTHER_WALLET, 0);
    let held_elsewhere = TokenPoolNote {
        reserved_by: Some([0xD2; 32]),
        ..owned(absent_account, 0, 5)
    };
    let tables = tables_with(&[held_elsewhere]).await;
    let released = tables
        .read()
        .await
        .with_token_pool_conn(|conn| {
            record_scan(conn, token().token_id, 50);
            sweep_all(
                conn,
                &BTreeSet::from([scanned_account]),
                &BTreeSet::from([token().token_id]),
                50,
                &BTreeSet::new(),
            )
        })
        .expect("sweep");
    assert!(released.is_empty());
    let held = tables
        .read()
        .await
        .with_token_pool_conn(|conn| notes(conn, absent_account, token().token_id))
        .expect("notes");
    assert_eq!(held[0].reserved_by, Some([0xD2; 32]));
}

/// A token pool scan answered from below the height the credit pool scan read
/// may not show a landed spend's token notes spent yet: its reservations are
/// kept until a scan reaches that height.
#[tokio::test]
async fn should_keep_reservations_until_the_token_scan_reaches_the_credit_scan() {
    let id = SubwalletId::new(WALLET, 0);
    let held = TokenPoolNote {
        reserved_by: Some([0xD3; 32]),
        ..owned(id, 0, 5)
    };
    let tables = tables_with(&[held]).await;
    let store = tables.read().await;
    let sweep_from = |scanned_from: u64| {
        store
            .with_token_pool_conn(|conn| {
                sweep_all(
                    conn,
                    &BTreeSet::from([id]),
                    &BTreeSet::from([token().token_id]),
                    scanned_from,
                    &BTreeSet::new(),
                )
            })
            .expect("sweep")
    };
    assert!(
        sweep_from(0).is_empty(),
        "a pool with no scan recorded keeps its holds"
    );
    store
        .with_token_pool_conn(|conn| {
            record_scan(conn, token().token_id, 99);
            Ok(())
        })
        .expect("scan");
    assert!(
        sweep_from(100).is_empty(),
        "a token scan one block behind the credit scan keeps its holds"
    );
    assert!(
        sweep_from(99).is_empty(),
        "a later credit scan answered from below keeps the height reached"
    );
    store
        .with_token_pool_conn(|conn| {
            record_scan(conn, token().token_id, 100);
            Ok(())
        })
        .expect("scan");
    assert_eq!(sweep_from(99), BTreeSet::from([(id, [0xD3; 32])]));
}

/// A pool cannot be unwatched while a spend holds some of the wallet's notes
/// there: watching it again would offer them to another spend. Once nothing
/// is held, it can.
#[tokio::test]
async fn should_refuse_to_unwatch_a_pool_while_a_spend_holds_its_notes() {
    let id = SubwalletId::new(WALLET, 0);
    let tables = tables_with(&[owned(id, 0, 5)]).await;
    let (_, reservation) =
        reserve_token_notes(&*tables.read().await, id, token().token_id, 5, 1).expect("reserve");
    let err = tables
        .read()
        .await
        .with_token_pool_conn(|conn| remove_watch(conn, WALLET, token().token_id))
        .expect_err("a spend holds a note");
    assert!(
        matches!(
            err,
            PlatformWalletError::TokenShieldedPoolSpendPending { .. }
        ),
        "got {err:?}"
    );
    assert_eq!(
        balance_rows(&tables, WALLET, &[0]).await[0].reserved_balance,
        5
    );

    // Another spend used the held note: the hold still links the sweep to
    // the activity it settles, so the pool stays watched.
    tables
        .read()
        .await
        .with_token_pool_conn(|conn| {
            conn.execute("UPDATE shielded_token_pool_notes SET spent = 1", [])
                .expect("mark spent");
            Ok(())
        })
        .expect("spent");
    let err = tables
        .read()
        .await
        .with_token_pool_conn(|conn| remove_watch(conn, WALLET, token().token_id))
        .expect_err("a spent note is still held");
    assert!(
        matches!(
            err,
            PlatformWalletError::TokenShieldedPoolSpendPending { .. }
        ),
        "got {err:?}"
    );

    reservation.release(&*tables.read().await);
    tables
        .read()
        .await
        .with_token_pool_conn(|conn| remove_watch(conn, WALLET, token().token_id))
        .expect("nothing held");
    assert!(balance_rows(&tables, WALLET, &[0]).await.is_empty());
}
