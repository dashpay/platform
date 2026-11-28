//! Shadow mode: a node applies the blocks it commits to a second database, on the other
//! storage engine, and holds what the two engines made of each block against each other.
//!
//! The shadow is a whole [`Platform`] of its own: its own Drive in its own directory, its
//! own platform state and caches. It reads Dash Core through the primary's client
//! ([`ShadowCore`]). It is given `InitChain` and every `FinalizeBlock` and nothing else, so
//! it executes decided blocks only, as a node does that has to execute a committed block
//! again. Every other request is the primary's alone, and no answer of the node is the
//! shadow's.
//!
//! At `FinalizeBlock`:
//!
//! 1. the shadow executes the block in its transaction;
//! 2. the two executions are compared ([`BlockLine`]): the app hash, the code and the fee
//!    of every transaction, the rest of the answer to `ProcessProposal`, and every fee
//!    calculation with the cost of each operation it was given;
//! 3. the shadow commits and its database is flushed;
//! 4. the primary finalizes the block, and its answer is the node's;
//! 5. the answers to `FinalizeBlock` and GroveDB's two roots are compared.
//!
//! The shadow commits first and is flushed before the primary commits, so that after a
//! crash it is at the primary's height or ahead of it. Tenderdash then gives the primary
//! the blocks it lost again, and the shadow skips the heights it holds.
//!
//! # What a divergence does
//!
//! It is counted, and what the comparison held when it found the divergence is written
//! to a directory of its own ([`Report`]), as far as the file system takes each write:
//! of the request, the answers and the lists of fee calculations, those it held; and a
//! checkpoint of the primary's database, and of the shadow's where it is open, as each
//! is then. Of the divergences in costs alone, which do not stop the shadow where the
//! node goes on, the two checkpoints are kept with the first
//! [`COST_DIVERGENCES_KEPT_WITH_THE_STORES`] that the report counts, by either setting
//! of [`OnDivergence`]: a later one is counted and written with everything else, and
//! says so. For a difference in the stored records, the two checkpoints that were
//! compared are left, and where the counts were written with the divergence, what the
//! comparison found is written, as far as the file system takes that. A divergence is
//! counted also where nothing of it could be written to that directory. Then, by
//! [`OnDivergence`]:
//!
//! - **The two executions differ (step 2).** Neither database has committed the block.
//!   `Continue`: the shadow applies no more blocks, unless only costs differ, and the
//!   primary finalizes the block. `Stop`: the request is answered with an error and the
//!   node ends; the primary has not committed the block.
//! - **The shadow fails in its execution or at its commit (steps 1 and 3).** The primary
//!   has not committed the block; the shadow has where it failed after its commit.
//!   `Continue`: the primary finalizes the block first. Where it commits it, the failure
//!   is a divergence and the shadow applies no more blocks. Where it refuses the request
//!   too, the failure is not counted and what was kept of it is removed: the request was
//!   no block for either. A difference in costs that was found in the two executions
//!   before it stays counted, with what was kept of it.
//!   `Stop`: the failure is counted, the request is answered with an error and the node
//!   ends, without the primary having been asked to finalize.
//! - **Found after both commits (step 5).** The block is committed on both, and the
//!   shadow applies no block after it. `Stop`: the node ends with the block committed.
//! - **Found in a comparison of the stored records**, by a thread that executes no block,
//!   while the node is at the block the comparison is of or at a later one. The shadow
//!   applies no block that begins after that thread has stopped it. `Stop`: the node
//!   ends.
//!
//! A difference in the answers to `InitChain` is found when the primary has made its
//! first state. `Continue`: the shadow applies no block. `Stop`: the request is answered
//! with an error and the node ends.
//!
//! A node that ends is told to shut down as it is at a signal. A `FinalizeBlock` that
//! begins after that is answered with an error; one that had begun is finished. Whether
//! an answer that is made while the node shuts down reaches Tenderdash is not promised.
//! What a node that is started again does goes by what the two databases then hold.
//!
//! A shadow that stopped stays stopped, also where the node ended with it: at the next
//! start its database is not opened, and the node goes on without it.
//!
//! # What is not the same as a node without a shadow
//!
//! A block takes longer, by what the shadow takes. The handlers' own metrics, their
//! durations and the gauges of the height, are written by both. For a node that
//! proposes, the fee calculations of `PrepareProposal` are recorded inside the time it has
//! for the transactions of a proposal, which is time the proposal does not have: a
//! proposer near that limit can leave a transaction out that it would have taken. A node
//! that does not propose answers what it would answer alone, until it ends where it is
//! configured to.
//!
//! Dash Core is asked by both: the shadow puts its questions to the primary's client when
//! it executes the block, at `FinalizeBlock`, where the primary put them at
//! `ProcessProposal`. A chain lock the block carries is submitted to Core by both. An
//! answer of Core that changes in between can give a divergence that is not an engine's.
//! The errors Core answers the shadow with are noted, the first 64 since the shadow last
//! began to execute a block, or since the node started where it began none. Where a
//! divergence is found at a block or at `InitChain` while any is noted, they are
//! written with what is kept of it, as far as that is written, and the height is
//! written down (`divergences_with_core_errors`). A difference in stored records is
//! found without a look at them, and an answer that differs and is no error is not told
//! from a difference between the engines.
//!
//! At `InitChain` the shadow asks Core for the first core height after the primary did,
//! and waits as the primary waits, with the node's answer, where Core then says that
//! the height is not chain locked or the fork not active, or where the node's clock is
//! then before the time of that core block.

mod compare;
mod core;
mod guard;
mod report;
#[cfg(feature = "shadow-store-compare")]
mod store;
#[cfg(any(test, feature = "mocks"))]
pub mod testing;
#[cfg(test)]
mod tests;
#[cfg(feature = "shadow-store-compare")]
mod walk;

pub use self::compare::{fees_text, BlockLine, Difference, DivergenceKind};
pub use self::core::ShadowCore;
pub use self::guard::a_panic_here_is_contained;
pub use self::report::{
    Counts, HeightRange, NotCompared, Report, Rewind, SkippedHeight, Stopped, StoreComparison,
    StoreComparisons, STATUS_FILE,
};
#[cfg(feature = "shadow-store-compare")]
pub use self::store::compare_stores;

use self::guard::{contained, guarded, Latches};
use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::abci::handler;
use crate::abci::handler::error::error_into_exception;
use crate::config::{OnDivergence, ShadowConfig};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::withdrawal::unsigned_withdrawal_txs_by_round::UnsignedWithdrawalTxsByRound;
use crate::rpc::core::CoreRPCLike;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use drive::config::StorageEngine;
use drive::fees::record::{record_fee_calculations, FeeCalculation};
use drive::grovedb::Transaction;
use prost::Message;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
use tenderdash_abci::proto::abci::tx_record::TxAction;

/// The Platform of the shadow.
pub type ShadowPlatform<'a, C> = Platform<ShadowCore<'a, C>>;

/// Why shadow mode did not start.
#[derive(Debug, thiserror::Error)]
pub enum ShadowError {
    /// The shadow's Platform was not opened.
    #[error("the shadow's database was not opened: {0}")]
    Platform(#[from] Error),
    /// The report was not opened or written.
    #[error("the report of shadow mode: {0}")]
    Report(#[from] std::io::Error),
    /// The two databases, or the configuration, are not ones shadow mode starts on.
    #[error("{0}")]
    Refused(String),
}

/// With how many of the divergences in costs alone that a report counts a checkpoint of
/// each database is kept. The shadow goes on after such a divergence where the node does,
/// and a node that ends at one is given the block again when it is started again; a
/// checkpoint holds the files of its database as they were, which the database then
/// cannot remove.
pub const COST_DIVERGENCES_KEPT_WITH_THE_STORES: u64 = 4;

/// Which storage engine the shadow is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowEngine {
    /// The engine the primary is not on.
    Other,
    /// The engine of the primary: a control, in which any difference comes from running
    /// two Platforms in one process and not from an engine.
    #[cfg(any(test, feature = "mocks"))]
    Same,
}

fn the_other(engine: StorageEngine) -> StorageEngine {
    match engine {
        StorageEngine::RocksDb => StorageEngine::LigninDb,
        StorageEngine::LigninDb => StorageEngine::RocksDb,
    }
}

/// Opens the Platform of the shadow where the configuration of `primary` turns shadow mode
/// on, and `None` where it does not, or where the report says that the shadow stopped in
/// an earlier run: a shadow that stopped is not opened again. Its configuration is the
/// primary's, with the shadow's directory, the other storage engine, and no directory for
/// rejected items.
///
/// To be called before the node serves anything: opening a Platform sets the protocol
/// version the process goes by, and this sets it back to the primary's.
pub fn open_shadow_platform<C: CoreRPCLike>(
    primary: &Platform<C>,
) -> Result<Option<ShadowPlatform<'_, C>>, ShadowError> {
    let settings = &primary.config.shadow;
    let (Some(path), Some(report)) = (settings.shadow_db_path.clone(), settings.report_path())
    else {
        return Ok(None);
    };
    // Before the shadow's database is opened, which makes its directory.
    #[cfg(feature = "shadow-store-compare")]
    if let Some(refused) =
        walk::inside_what_a_start_empties(&primary.config.db_path, &path, &report)
    {
        return Err(ShadowError::Refused(refused));
    }
    if let Some(stopped) = Report::stopped_in(&report)? {
        tracing::warn!(
            reason = stopped.reason,
            "the shadow stopped applying blocks in an earlier run: its database is not \
             opened; to begin again: {REMEDY}"
        );
        return Ok(None);
    }
    open_shadow_platform_at(primary, path, ShadowEngine::Other, None).map(Some)
}

/// `alter` changes the configuration of the shadow's Platform after it is made from the
/// primary's: a test's way to a shadow that is configured otherwise.
fn open_shadow_platform_at<C: CoreRPCLike>(
    primary: &Platform<C>,
    path: std::path::PathBuf,
    engine: ShadowEngine,
    alter: Option<fn(&mut crate::config::PlatformConfig)>,
) -> Result<ShadowPlatform<'_, C>, ShadowError> {
    if path == primary.config.db_path {
        return Err(ShadowError::Refused(format!(
            "SHADOW_DB_PATH is the directory of the node's own database, {}",
            path.display()
        )));
    }
    let mut config = primary.config.clone();
    config.db_path = path.clone();
    config.rejections_path = None;
    config.shadow = Default::default();
    config.drive.grovedb_storage_engine = match engine {
        ShadowEngine::Other => the_other(primary.drive.storage_engine()),
        #[cfg(any(test, feature = "mocks"))]
        ShadowEngine::Same => primary.drive.storage_engine(),
    };
    if let Some(alter) = alter {
        alter(&mut config);
    }
    // A database that holds no block yet begins at the protocol version the primary began
    // at; one that holds a block reads its own.
    let primary_state = primary.state.load();
    let initial_protocol_version = primary_state.current_protocol_version_in_consensus();
    let version_of_the_process = primary_state.current_platform_version()?;
    let shadow = Platform::open_with_client(
        path,
        Some(config),
        ShadowCore::of(&primary.core_rpc),
        Some(initial_protocol_version),
    );
    // Opening a Platform that holds a block publishes its protocol version to the
    // process. A shadow that is ahead of the primary across an upgrade holds another:
    // the process goes by the primary's.
    PlatformVersion::set_current(version_of_the_process);
    Ok(shadow?)
}

/// Records the fee calculations `run` makes on this thread. In a build for tests it also
/// counts the ones `drive` makes meanwhile on every thread, and the two have to agree: a
/// calculation of a block's execution on another thread would not be in the record.
fn recorded<R>(drive: &drive::drive::Drive, run: impl FnOnce() -> R) -> (R, Vec<FeeCalculation>) {
    #[cfg(any(test, feature = "mocks"))]
    {
        use drive::fees::record::every_thread::count_fee_calculations;
        let ((returned, fees), on_every_thread) =
            count_fee_calculations(drive, || record_fee_calculations(run));
        assert_eq!(
            fees.len() as u64,
            on_every_thread,
            "a fee calculation of a block's execution ran on another thread than the one \
             that records them"
        );
        (returned, fees)
    }
    #[cfg(not(any(test, feature = "mocks")))]
    {
        let _ = drive;
        record_fee_calculations(run)
    }
}

/// The application of the shadow: what the handlers of block execution need of one.
struct ShadowApplication<'a, C> {
    platform: &'a ShadowPlatform<'a, C>,
    transaction: RwLock<Option<Transaction<'a>>>,
    block_execution_context: RwLock<Option<BlockExecutionContext>>,
    unsigned_withdrawal_txs_by_round: RwLock<UnsignedWithdrawalTxsByRound>,
}

impl<'a, C> PlatformApplication<ShadowCore<'a, C>> for ShadowApplication<'a, C> {
    fn platform(&self) -> &Platform<ShadowCore<'a, C>> {
        self.platform
    }
}

impl<C> BlockExecutionApplication for ShadowApplication<'_, C> {
    fn block_execution_context(&self) -> &RwLock<Option<BlockExecutionContext>> {
        &self.block_execution_context
    }

    fn unsigned_withdrawal_txs_by_round(&self) -> &RwLock<UnsignedWithdrawalTxsByRound> {
        &self.unsigned_withdrawal_txs_by_round
    }
}

impl<'a, C> TransactionalApplication<'a> for ShadowApplication<'a, C> {
    fn start_transaction(&self) {
        let transaction = self.platform.drive.grove.start_transaction();
        self.transaction.write().unwrap().replace(transaction);
    }

    fn transaction(&self) -> &RwLock<Option<Transaction<'a>>> {
        &self.transaction
    }

    fn commit_transaction(&self, platform_version: &PlatformVersion) -> Result<(), Error> {
        let transaction = self
            .transaction
            .write()
            .unwrap()
            .take()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "trying to commit a transaction, but we are not in one",
            )))?;

        self.platform
            .drive
            .commit_transaction(transaction, &platform_version.drive)
            .map_err(Error::Drive)
    }
}

/// What an operator does to begin shadow mode again on two databases it does not start
/// on.
const REMEDY: &str = "stop the node, make the shadow's database from the primary's with the \
                      migrator (or remove both and sync again), remove the report's status \
                      file, and start";

/// Whether the comparison begins on a primary and a shadow at these heights, each with
/// the app hash of the last block it holds, if it holds one: the shadow is not behind,
/// and at the primary's height it holds the primary's state.
fn begins_on(
    primary: (u64, Option<[u8; 32]>),
    shadow: (u64, Option<[u8; 32]>),
) -> Result<(), String> {
    let ((primary_height, primary_app_hash), (shadow_height, shadow_app_hash)) = (primary, shadow);
    if shadow_height < primary_height {
        return Err(format!(
            "the shadow's database is at height {shadow_height} and the primary's at \
             {primary_height}: the shadow is behind and cannot be given the blocks between; \
             {REMEDY}"
        ));
    }
    if shadow_height == primary_height && primary_app_hash != shadow_app_hash {
        let said = |app_hash: Option<[u8; 32]>| match app_hash {
            Some(app_hash) => hex::encode(app_hash),
            None => "none, for it holds no block".to_string(),
        };
        return Err(format!(
            "at height {primary_height} the primary's app hash is {} and the shadow's is {}: \
             the two databases do not hold the same state; {REMEDY}",
            said(primary_app_hash),
            said(shadow_app_hash),
        ));
    }
    Ok(())
}

/// Brings the counts to a start of the node, whose primary is at `primary_height`.
///
/// What the start sees is written down, and nothing is said of how it came about: where
/// the primary's database is above the height of the counts of an earlier start, the
/// heights between, from the first of the chain on; where it is below, both heights.
fn at_a_start(
    counts: &mut Counts,
    primary_height: u64,
    shadow_height: u64,
    stopped: &Option<Stopped>,
    genesis_height: u64,
) {
    // The counts of a first start hold no height of an earlier one.
    if counts.starts > 1 {
        let first = (counts.height + 1).max(genesis_height);
        if first <= primary_height {
            counts.blocks_uncounted.push(HeightRange {
                height: first,
                through: primary_height,
            });
        }
        if counts.height > primary_height {
            counts.rewinds.push(Rewind {
                counted: counts.height,
                held: primary_height,
            });
        }
    }
    counts.height = primary_height;
    counts.shadow_height = shadow_height;
    counts.shadow_stopped = stopped.clone();
    counts.shadow_stop_pending = None;
}

/// The height of the last block a Platform holds and its app hash, where it holds one.
fn held_by(state: &PlatformState) -> (u64, Option<[u8; 32]>) {
    let app_hash = state
        .last_committed_block_info()
        .is_some()
        .then(|| state.last_committed_block_app_hash().unwrap_or_default());
    (state.last_committed_block_height(), app_hash)
}

/// The primary's execution of a proposal it accepted.
struct Executed {
    height: i64,
    round: i32,
    block_hash: Vec<u8>,
    answer: proto::ResponseProcessProposal,
    /// The fee calculations of the block. `None` where the answer came from a proposal
    /// the primary prepared and the calculations of that preparation are not held.
    fees: Option<Vec<FeeCalculation>>,
}

impl Executed {
    fn is_of(&self, request: &proto::RequestFinalizeBlock) -> bool {
        self.height == request.height
            && self.round == request.round
            && self.block_hash == request.hash
    }
}

/// The fee calculations of the block of a proposal the primary prepared itself: those of
/// the preparation without the ones of the transactions it left out.
struct Prepared {
    height: i64,
    round: i32,
    app_hash: Vec<u8>,
    fees: Vec<FeeCalculation>,
}

#[derive(Default)]
struct PrimarySide {
    prepared: Option<Prepared>,
    executed: Option<Executed>,
    milliseconds: u64,
}

impl PrimarySide {
    /// The execution an answer of the primary's handler is of. `from_prepared` says that
    /// the handler answered from the proposal it prepared and executed nothing: the fee
    /// calculations are then those of the preparation, where they are held for this
    /// height, round and app hash, and not the ones recorded around the handler, which
    /// are none.
    fn executed(
        &self,
        (height, round, block_hash): (i64, i32, Vec<u8>),
        answer: proto::ResponseProcessProposal,
        recorded: Vec<FeeCalculation>,
        from_prepared: bool,
    ) -> Executed {
        let fees = match from_prepared {
            false => Some(recorded),
            true => self
                .prepared
                .as_ref()
                .filter(|prepared| {
                    prepared.height == height
                        && prepared.round == round
                        && prepared.app_hash == answer.app_hash
                })
                .map(|prepared| prepared.fees.clone()),
        };
        Executed {
            height,
            round,
            block_hash,
            answer,
            fees,
        }
    }
}

fn milliseconds(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn exception(error: String) -> proto::ResponseException {
    proto::ResponseException { error }
}

/// What became of a block the primary committed.
enum Outcome {
    /// Both executed it, and nothing differs.
    Compared,
    /// A divergence was found at it.
    Diverged,
    /// Both executed it and it was not compared in full: with what was not.
    NotCompared(String),
    /// The shadow held it already.
    Skipped { app_hash_compared: bool },
    /// The shadow applies no blocks.
    WithoutShadow,
}

/// A divergence that was found and is not counted yet, with what was kept of it.
struct Found {
    kind: DivergenceKind,
    /// Whether errors of Dash Core were noted when it was found.
    core_errors: bool,
    /// The directory that keeps it, where it was made.
    kept: Option<report::Kept>,
}

impl Found {
    fn counted_in(&self, counts: &mut Counts, height: u64) {
        counts.divergence(height, self.kind);
        if self.core_errors {
            counts.divergences_with_core_errors.push(height);
        }
    }
}

/// What is written in the same write as the count of a block.
#[derive(Default)]
struct With<'f> {
    /// The stop of the shadow that the block brought.
    stop: Option<Stopped>,
    /// The divergence that was found at the block.
    found: Option<&'f Found>,
}

/// What is kept of a divergence, as far as it is at hand where the divergence is found:
/// each part by itself, so that what one side made is kept where the other made nothing.
#[derive(Default)]
struct Evidence<'e> {
    request: Option<&'e proto::RequestFinalizeBlock>,
    primary: Option<&'e Executed>,
    shadow_answer: Option<&'e proto::ResponseProcessProposal>,
    shadow_fees: Option<&'e [FeeCalculation]>,
    lines: Option<(&'e BlockLine, &'e BlockLine)>,
    primary_final: Option<&'e proto::ResponseFinalizeBlock>,
    shadow_final: Option<&'e proto::ResponseFinalizeBlock>,
    files: Vec<(&'static str, Vec<u8>)>,
    notes: Vec<String>,
}

impl Evidence<'_> {
    fn files(mut self) -> (Vec<(&'static str, Vec<u8>)>, Vec<String>) {
        let mut files = std::mem::take(&mut self.files);
        if let Some(request) = self.request {
            files.push(("finalize-block-request.bin", request.encode_to_vec()));
        }
        if let Some(primary) = self.primary {
            files.push((
                "primary-process-proposal.bin",
                primary.answer.encode_to_vec(),
            ));
            if let Some(fees) = &primary.fees {
                files.push(("primary-fees.txt", fees_text(fees).into_bytes()));
            }
        }
        if let Some(answer) = self.shadow_answer {
            files.push(("shadow-process-proposal.bin", answer.encode_to_vec()));
        }
        if let Some(fees) = self.shadow_fees {
            files.push(("shadow-fees.txt", fees_text(fees).into_bytes()));
        }
        if let Some((primary, shadow)) = self.lines {
            files.push(("primary.json", primary.json().into_bytes()));
            files.push(("shadow.json", shadow.json().into_bytes()));
        }
        if let Some(answer) = self.primary_final {
            files.push(("primary-finalize-block.bin", answer.encode_to_vec()));
        }
        if let Some(answer) = self.shadow_final {
            files.push(("shadow-finalize-block.bin", answer.encode_to_vec()));
        }
        (files, self.notes)
    }
}

/// GroveDB's root of a Platform's database, as committed.
fn root_of<C>(platform: &Platform<C>) -> Result<Vec<u8>, String> {
    let platform_version = platform
        .state
        .load()
        .current_platform_version()
        .map_err(|error| error.to_string())?;
    platform
        .drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .map(|root| root.to_vec())
        .map_err(|error| error.to_string())
}

/// The shadow of a consensus application.
pub struct Shadow<'a, C> {
    /// `None` where the shadow stopped in an earlier run: its database is not open, and
    /// the blocks the primary commits are counted as without a shadow.
    app: Option<ShadowApplication<'a, C>>,
    report: Arc<Report>,
    on_divergence: OnDivergence,
    latches: Arc<Latches>,
    #[cfg_attr(not(feature = "shadow-store-compare"), allow(dead_code))]
    store_compare_interval: Option<Duration>,
    #[cfg_attr(not(feature = "shadow-store-compare"), allow(dead_code))]
    last_store_comparison: Mutex<Instant>,
    primary: Mutex<PrimarySide>,
    /// What a test plants in the next block.
    #[cfg(any(test, feature = "mocks"))]
    planted: Mutex<Option<testing::Fault>>,
    /// Where a test holds the comparisons of stored records.
    #[cfg(any(test, feature = "mocks"))]
    #[cfg_attr(not(feature = "shadow-store-compare"), allow(dead_code))]
    gate: Mutex<Option<Arc<testing::Gate>>>,
}

impl<C> std::fmt::Debug for Shadow<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Shadow>")
    }
}

impl<'a, C: CoreRPCLike> Shadow<'a, C> {
    /// The shadow of the application of `primary`, where its configuration turns shadow
    /// mode on, on the Platform `shadow` that [`open_shadow_platform`] gave for it.
    /// `end_the_node` is called, once, where the configuration ends the node at a
    /// divergence.
    ///
    /// `None` where shadow mode is off. Where `shadow` is `None` because the shadow
    /// stopped in an earlier run, the shadow of the application applies nothing and
    /// counts the blocks the primary commits.
    ///
    /// Refused where the shadow is behind the primary, or at its height with another app
    /// hash: the comparison begins from two databases that hold the same blocks. Ahead of
    /// the primary it is not refused, and skips the heights it holds.
    pub fn for_application(
        primary: &'a Platform<C>,
        shadow: Option<&'a ShadowPlatform<'a, C>>,
        end_the_node: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Option<Self>, ShadowError> {
        if primary.config.shadow.shadow_db_path.is_none() {
            return Ok(None);
        }
        Self::with_settings(primary, shadow, &primary.config.shadow, end_the_node).map(Some)
    }

    fn with_settings(
        primary: &'a Platform<C>,
        shadow: Option<&'a ShadowPlatform<'a, C>>,
        settings: &ShadowConfig,
        end_the_node: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, ShadowError> {
        let Some(report_path) = settings.report_path() else {
            return Err(ShadowError::Refused(
                "shadow mode is not turned on: SHADOW_DB_PATH is not set".to_string(),
            ));
        };
        #[cfg(feature = "shadow-store-compare")]
        if let Some(shadow_directory) = &settings.shadow_db_path {
            let refused = walk::inside_what_a_start_empties(
                &primary.config.db_path,
                shadow_directory,
                &report_path,
            );
            if let Some(refused) = refused {
                return Err(ShadowError::Refused(refused));
            }
        }
        if settings.shadow_store_compare_interval_s > 0 && !cfg!(feature = "shadow-store-compare") {
            return Err(ShadowError::Refused(
                "SHADOW_STORE_COMPARE_INTERVAL_S is set and this build has no comparison of \
                 stored records: a build with the feature `shadow-store-compare` has it"
                    .to_string(),
            ));
        }
        let primary_engine = primary.drive.storage_engine();
        let shadow_engine = match shadow {
            Some(shadow) => shadow.drive.storage_engine(),
            None => Report::engines_in(&report_path)?
                .map(|(_, shadow_engine)| shadow_engine)
                .unwrap_or_else(|| the_other(primary_engine).to_string())
                .parse()
                .map_err(ShadowError::Refused)?,
        };
        // What the report holds is read, and nothing is written before the start is
        // decided: a start that is refused leaves no counts behind.
        let before = Report::counts_in(&report_path)?.unwrap_or_default();
        let (primary_height, primary_app_hash) = held_by(&primary.state.load());
        // A stop that a comparison of stored records asked for and that no block wrote
        // down before the node stopped is a stop.
        let stopped = before.shadow_stopped.clone().or_else(|| {
            before.shadow_stop_pending.clone().map(|reason| Stopped {
                height: primary_height.max(before.shadow_height) + 1,
                reason: format!(
                    "{reason}; written down at a start, with the shadow's database not opened"
                ),
            })
        });

        let shadow_height = match (&stopped, shadow) {
            (Some(stopped), _) => {
                tracing::warn!(
                    height = stopped.height,
                    reason = stopped.reason,
                    "the shadow stopped applying blocks in an earlier run and stays stopped; to \
                     begin again: {REMEDY}"
                );
                before.shadow_height
            }
            (None, None) => {
                return Err(ShadowError::Refused(
                    "shadow mode is on and the shadow's database is not open".to_string(),
                ))
            }
            (None, Some(shadow)) => {
                let (shadow_height, shadow_app_hash) = held_by(&shadow.state.load());
                begins_on(
                    (primary_height, primary_app_hash),
                    (shadow_height, shadow_app_hash),
                )
                .map_err(ShadowError::Refused)?;
                if shadow_height > primary_height {
                    tracing::warn!(
                        primary_height,
                        shadow_height,
                        "the shadow is ahead of the primary and skips the heights it holds"
                    );
                }
                shadow_height
            }
        };

        // The report is opened and the start is written in one write.
        let genesis_height = primary.config.abci.genesis_height;
        let report = Report::open_with(
            &report_path,
            &primary_engine.to_string(),
            &shadow_engine.to_string(),
            |counts| {
                at_a_start(
                    counts,
                    primary_height,
                    shadow_height,
                    &stopped,
                    genesis_height,
                )
            },
        )?;
        tracing::info!(
            primary = %primary_engine,
            shadow = %shadow_engine,
            primary_height,
            shadow_height,
            report = %report_path.display(),
            on_divergence = ?settings.shadow_on_divergence,
            stopped = stopped.is_some(),
            "shadow mode is on"
        );

        let applying = shadow.filter(|_| stopped.is_none());
        #[cfg(feature = "shadow-store-compare")]
        {
            walk::remove_what_a_comparison_left(&primary.config.db_path, primary_engine);
            // The shadow's directory by its setting: it is cleaned where the shadow is
            // not opened too.
            if let Some(shadow_directory) = &settings.shadow_db_path {
                walk::remove_what_a_comparison_left(shadow_directory, shadow_engine);
            }
        }
        Ok(Self {
            app: applying.map(|platform| ShadowApplication {
                platform,
                transaction: Default::default(),
                block_execution_context: Default::default(),
                unsigned_withdrawal_txs_by_round: Default::default(),
            }),
            report: Arc::new(report),
            on_divergence: settings.shadow_on_divergence,
            latches: Arc::new(Latches::new(stopped.is_some(), end_the_node)),
            store_compare_interval: (settings.shadow_store_compare_interval_s > 0)
                .then(|| Duration::from_secs(settings.shadow_store_compare_interval_s)),
            last_store_comparison: Mutex::new(Instant::now()),
            primary: Default::default(),
            #[cfg(any(test, feature = "mocks"))]
            planted: Mutex::new(None),
            #[cfg(any(test, feature = "mocks"))]
            gate: Mutex::new(None),
        })
    }

    /// Whether a test planted `fault` in this block. It is planted once.
    #[cfg(any(test, feature = "mocks"))]
    fn planted(&self, fault: testing::Fault) -> bool {
        let mut planted = self
            .planted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *planted == Some(fault) {
            *planted = None;
            return true;
        }
        false
    }

    /// Says that the last comparison of the stored records began `ago`, so that a test
    /// need not wait for an interval to pass.
    #[cfg(any(test, feature = "mocks"))]
    pub fn the_last_store_comparison_began(&self, ago: Duration) {
        if let Some(then) = Instant::now().checked_sub(ago) {
            *self
                .last_store_comparison
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = then;
        }
    }

    /// Plants `fault` in the next block that both engines execute.
    #[cfg(any(test, feature = "mocks"))]
    pub fn plant(&self, fault: testing::Fault) {
        *self
            .planted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(fault);
    }

    /// Holds every comparison of stored records that begins from now on at its start,
    /// until the gate that is returned is opened.
    #[cfg(any(test, feature = "mocks"))]
    pub fn hold_the_store_comparisons(&self) -> Arc<testing::Gate> {
        let gate = Arc::new(testing::Gate::default());
        *self
            .gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::clone(&gate));
        gate
    }

    /// The Platform of the shadow, where its database is open.
    pub fn platform(&self) -> Option<&'a ShadowPlatform<'a, C>> {
        self.app.as_ref().map(|app| app.platform)
    }

    /// The report.
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// The application of the shadow, where it applies blocks.
    fn applying(&self) -> Option<&ShadowApplication<'a, C>> {
        self.app.as_ref().filter(|_| self.latches.applies())
    }

    fn primary_side(&self) -> std::sync::MutexGuard<'_, PrimarySide> {
        self.primary
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn written(&self, written: std::io::Result<()>) {
        if let Err(error) = written {
            tracing::error!(?error, "the report of shadow mode was not written");
        }
    }

    /// Counts a block the primary committed, once, as what became of it. `line` is the
    /// primary's line of it, where its execution by the primary was seen.
    fn committed(
        &self,
        height: u64,
        shadow_holds_it: bool,
        line: Option<&BlockLine>,
        outcome: Outcome,
    ) {
        self.count(height, shadow_holds_it, line, outcome, With::default())
    }

    /// The same, with the divergence that was found at the block and the stop of the
    /// shadow that it brought written in the same write: a node that stops there has
    /// written all of it or none.
    fn count(
        &self,
        height: u64,
        shadow_holds_it: bool,
        line: Option<&BlockLine>,
        outcome: Outcome,
        with: With<'_>,
    ) {
        if let Some(line) = line {
            self.written(self.report.block(line));
        }
        self.written(self.report.change(|counts| {
            counts.first_counted_height.get_or_insert(height);
            counts.height = height;
            if shadow_holds_it {
                counts.shadow_height = counts.shadow_height.max(height);
            }
            match outcome {
                Outcome::Compared => {
                    counts.blocks_compared += 1;
                    counts.first_compared_height.get_or_insert(height);
                }
                Outcome::Diverged => counts.blocks_diverged += 1,
                Outcome::NotCompared(reason) => counts.blocks_not_compared.push(NotCompared {
                    height,
                    through: height,
                    reason,
                }),
                Outcome::Skipped { app_hash_compared } => {
                    counts.heights_skipped.push(SkippedHeight {
                        height,
                        app_hash_compared,
                    })
                }
                Outcome::WithoutShadow => counts.blocks_without_shadow += 1,
            }
            if let Some(found) = with.found {
                found.counted_in(counts, height);
            }
            if let Some(stop) = with.stop {
                counts.shadow_stop_pending = None;
                counts.shadow_stopped.get_or_insert(stop);
            }
        }));
    }

    /// Writes down that the shadow's database holds the block at `height`, which the
    /// primary has not committed yet.
    fn shadow_holds(&self, height: u64) {
        self.written(self.report.change(|counts| {
            counts.shadow_height = counts.shadow_height.max(height);
        }));
    }

    /// Drops what the shadow holds of a block it did not commit.
    fn forget_the_block(&self, app: &ShadowApplication<'a, C>) {
        // A lock that a panic of the shadow poisoned is left as it is: nothing takes it
        // again.
        if let Ok(mut context) = app.block_execution_context.write() {
            context.take();
        }
        if let Ok(mut transaction) = app.transaction.write() {
            transaction.take();
        }
    }

    /// The shadow applies no more blocks: what it holds of a block it did not commit is
    /// dropped, and it stays where it is. The caller writes the stop down.
    fn retire(&self, app: &ShadowApplication<'a, C>) {
        self.latches.stop();
        self.forget_the_block(app);
    }

    /// The shadow applies no more blocks, which is written down. `height` is the first
    /// block it does not apply.
    fn stop_the_shadow(&self, app: &ShadowApplication<'a, C>, height: u64, reason: String) {
        self.retire(app);
        self.write_the_stop(height, reason);
    }

    /// Writes down that the shadow applies no block from `height` on. A stop that is
    /// written down already stays as it is.
    fn write_the_stop(&self, height: u64, reason: String) {
        tracing::error!(height, reason, "the shadow applies no more blocks");
        self.written(self.report.change(|counts| {
            counts.shadow_stop_pending = None;
            if counts.shadow_stopped.is_none() {
                counts.shadow_stopped = Some(Stopped { height, reason });
            }
        }));
    }

    /// Keeps a checkpoint of each database in `kept`, as each is when this is called.
    fn keep_the_stores<A>(&self, primary: &A, kept: &report::Kept) -> Vec<String>
    where
        A: PlatformApplication<C>,
    {
        let mut said = Vec::new();
        let mut keep = |name: &str, grove: &drive::grovedb::GroveDb| {
            let path = kept.directory().join(format!("{name}-store"));
            let made = contained(|| grove.create_checkpoint(&path))
                .and_then(|made| made.map_err(|error| error.to_string()));
            match made {
                Ok(()) => said.push(format!("{name} store kept at {}", path.display())),
                Err(error) => said.push(format!("{name} store not kept: {error}")),
            }
        };
        keep("primary", &primary.platform().drive.grove);
        if let Some(app) = &self.app {
            keep("shadow", &app.platform.drive.grove);
        }
        said
    }

    /// Keeps what a divergence was found in, with the stores as they are when this is
    /// called, as far as each can be written, and counts nothing. The errors of Dash
    /// Core that are noted then are written with it.
    ///
    /// A divergence in costs alone is kept without the stores where the report counts
    /// [`COST_DIVERGENCES_KEPT_WITH_THE_STORES`] of them already, whatever the node does
    /// at a divergence. Where it goes on, the shadow goes on after one, so a cost that
    /// differs in every block would keep two checkpoints for every block. Where it ends,
    /// neither database has committed the block: a node that is started again is given
    /// the block again, and a cost that differs in it again is counted again.
    fn evidence_kept<A>(
        &self,
        primary: &A,
        height: u64,
        kind: DivergenceKind,
        differences: &[Difference],
        evidence: Evidence<'_>,
    ) -> Found
    where
        A: PlatformApplication<C>,
    {
        let core_errors = self
            .app
            .as_ref()
            .map(|app| app.platform.core_rpc.errors_noted())
            .unwrap_or_default();
        let mut found = Found {
            kind,
            core_errors: !core_errors.is_empty(),
            kept: None,
        };
        let kept = match self.report.kept_at(height) {
            Ok(kept) => kept,
            Err(error) => {
                tracing::error!(?error, height, "a divergence was not kept");
                for difference in differences {
                    tracing::error!("{difference}");
                }
                return found;
            }
        };
        let (files, mut notes) = evidence.files();
        for (name, bytes) in files {
            self.written(kept.file(name, &bytes));
        }
        notes.extend(core_errors.iter().map(|error| {
            format!("Dash Core answered the shadow with an error before this was found: {error}")
        }));
        let in_costs_before = match kind {
            DivergenceKind::Cost => self.report.divergences_of(kind),
            _ => 0,
        };
        if in_costs_before < COST_DIVERGENCES_KEPT_WITH_THE_STORES {
            notes.extend(self.keep_the_stores(primary, &kept));
        } else {
            notes.push(format!(
                "the stores are not kept: the report counts {in_costs_before} divergences in \
                 costs alone before this one, and the stores are kept with the first \
                 {COST_DIVERGENCES_KEPT_WITH_THE_STORES}"
            ));
        }
        self.written(kept.differences(differences, &notes));
        tracing::error!(
            height,
            kind = kind.name(),
            kept = %kept.directory().display(),
            "the two storage engines diverge"
        );
        found.kept = Some(kept);
        found
    }

    /// Keeps what a divergence was found in and counts it.
    fn keep<A>(
        &self,
        primary: &A,
        height: u64,
        kind: DivergenceKind,
        differences: &[Difference],
        evidence: Evidence<'_>,
    ) where
        A: PlatformApplication<C>,
    {
        let found = self.evidence_kept(primary, height, kind, differences, evidence);
        self.written(
            self.report
                .change(|counts| found.counted_in(counts, height)),
        );
    }

    /// Ends the node for a divergence at `height`, and gives the error a `FinalizeBlock`
    /// is answered with from then on.
    fn end(&self, height: u64) -> proto::ResponseException {
        self.ended_exception(self.latches.end(height))
    }

    fn ended_exception(&self, height: u64) -> proto::ResponseException {
        exception(format!(
            "shadow mode: the two storage engines diverge at height {height} and the \
             configuration ends the node at a divergence (SHADOW_ON_DIVERGENCE=stop); see {}",
            self.report.directory().display()
        ))
    }

    /// After a failure of the shadow at a block the primary has not committed, with what
    /// was found kept and not counted yet.
    ///
    /// Where the node is to end, the failure is counted and the node ends with the block
    /// not committed by the primary. Else the primary finalizes the block first. Where it
    /// refuses the request, the request was no block for it either: the failure is not
    /// counted, what was kept of it is removed, the shadow is not stopped, and what it
    /// holds of the block is left as its handler left it. Where it commits the
    /// block, the divergence, the block and the stop of the shadow are written in one
    /// write.
    #[allow(clippy::too_many_arguments)]
    fn after_a_failure_of_the_shadow<A>(
        &self,
        primary: &A,
        app: &ShadowApplication<'a, C>,
        request: proto::RequestFinalizeBlock,
        found: Found,
        line: Option<&BlockLine>,
        shadow_holds_it: bool,
        stop: Stopped,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException>
    where
        A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    {
        let height = u64::try_from(request.height).unwrap_or_default();
        if self.on_divergence == OnDivergence::Stop {
            self.written(
                self.report
                    .change(|counts| found.counted_in(counts, height)),
            );
            return Err(self.end(height));
        }
        // What the shadow holds of the block stays as its handler left it until the
        // primary has answered: where the primary refuses the request, the next one finds
        // on the shadow what it finds on the primary, the transaction of `InitChain` at
        // the first block included.
        let answer = match handler::finalize_block(primary, request) {
            Ok(answer) => answer,
            Err(error) => {
                if let Some(kept) = found.kept {
                    kept.discard();
                }
                return Err(error_into_exception(error));
            }
        };
        self.retire(app);
        tracing::error!(
            height = stop.height,
            reason = stop.reason,
            "the shadow applies no more blocks"
        );
        let with = With {
            stop: Some(stop),
            found: Some(&found),
        };
        self.count(height, shadow_holds_it, line, Outcome::Diverged, with);
        Ok(answer)
    }

    /// After a divergence that was found when both had committed the block at `height`:
    /// the shadow applies no block after it. Returns the stop, which the caller writes
    /// down with the count of the block.
    fn retired_after(
        &self,
        app: &ShadowApplication<'a, C>,
        height: u64,
        reason: String,
    ) -> Stopped {
        self.retire(app);
        tracing::error!(
            height = height + 1,
            reason,
            "the shadow applies no more blocks"
        );
        Stopped {
            height: height + 1,
            reason,
        }
    }

    /// After the same divergence, when the block is counted: the node ends after the
    /// block where it is configured to.
    fn ended_after(&self, height: u64) {
        if self.on_divergence == OnDivergence::Stop {
            self.latches.end(height);
        }
    }

    /// The difference that an error or a panic of the shadow at `step` is.
    fn shadow_error(height: u64, step: &str, error: &str) -> Vec<Difference> {
        vec![Difference {
            height,
            field: "shadow_error",
            this: "\"ok\"".to_string(),
            other: serde_json::to_string(&format!("{step}: {error}")).expect("a text is JSON"),
        }]
    }

    /// `InitChain`: the primary's, and the shadow's where its database holds no block.
    pub fn init_chain<A>(
        &self,
        primary: &A,
        request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, proto::ResponseException>
    where
        A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    {
        let answer = handler::init_chain(primary, request.clone()).map_err(error_into_exception)?;
        let Some(app) = self.applying() else {
            return Ok(answer);
        };
        if app
            .platform
            .state
            .load()
            .last_committed_block_info()
            .is_some()
        {
            return Ok(answer);
        }
        let height = u64::try_from(request.initial_height).unwrap_or_default();
        let request_bytes = request.encode_to_vec();
        let of_shadow = guarded(|| handler::init_chain(app, request));
        #[cfg(any(test, feature = "mocks"))]
        let of_shadow = of_shadow.map(|mut of_shadow| {
            if self.planted(testing::Fault::ShadowInitChainAnswer) {
                of_shadow.app_hash.push(0);
            }
            of_shadow
        });
        let mut evidence = Evidence {
            files: vec![
                ("init-chain-request.bin", request_bytes),
                ("primary-init-chain.bin", answer.encode_to_vec()),
            ],
            ..Default::default()
        };
        let (kind, differences, reason) = match of_shadow {
            Ok(of_shadow) if of_shadow == answer => return Ok(answer),
            Ok(of_shadow) => {
                evidence
                    .files
                    .push(("shadow-init-chain.bin", of_shadow.encode_to_vec()));
                let differences = vec![Difference {
                    height,
                    field: "init_chain_sha256",
                    this: format!("\"{}\"", compare::sha256_hex(&answer.encode_to_vec())),
                    other: format!("\"{}\"", compare::sha256_hex(&of_shadow.encode_to_vec())),
                }];
                (
                    DivergenceKind::State,
                    differences,
                    "InitChain differs".to_string(),
                )
            }
            Err(error) => (
                DivergenceKind::ShadowError,
                Self::shadow_error(height, "InitChain", &error),
                format!("the shadow failed at InitChain: {error}"),
            ),
        };
        self.keep(primary, height, kind, &differences, evidence);
        match self.on_divergence {
            OnDivergence::Stop => Err(self.end(height)),
            OnDivergence::Continue => {
                self.stop_the_shadow(app, height, reason);
                Ok(answer)
            }
        }
    }

    /// `PrepareProposal`: the primary's alone. While the shadow applies blocks, its fee
    /// calculations are recorded, and those of the transactions it keeps are held for
    /// the comparison of the block, should it be committed.
    pub fn prepare_proposal<A>(
        &self,
        primary: &A,
        request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, proto::ResponseException>
    where
        A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    {
        if self.applying().is_none() {
            return handler::prepare_proposal(primary, request).map_err(error_into_exception);
        }
        let (height, round) = (request.height, request.round);
        let began = Instant::now();
        let (answer, fees) = recorded(&primary.platform().drive, || {
            handler::prepare_proposal(primary, request)
        });
        let answer = answer.map_err(error_into_exception)?;
        // The records of the transactions that were executed come first and in their
        // order; a transaction is in the block where its record says it is unmodified.
        let kept: Vec<bool> = answer
            .tx_records
            .iter()
            .map(|record| record.action == TxAction::Unmodified as i32)
            .collect();
        let mut side = self.primary_side();
        side.milliseconds = milliseconds(began);
        side.prepared = Some(Prepared {
            height,
            round,
            app_hash: answer.app_hash.clone(),
            fees: compare::of_the_block(&fees, &kept),
        });
        Ok(answer)
    }

    /// `ProcessProposal`: the primary's alone. While the shadow applies blocks, the answer
    /// and the fee calculations of a proposal it accepts are held for the comparison of
    /// the block, should it be committed.
    pub fn process_proposal<A>(
        &self,
        primary: &A,
        request: proto::RequestProcessProposal,
    ) -> Result<proto::ResponseProcessProposal, proto::ResponseException>
    where
        A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    {
        if self.applying().is_none() {
            return handler::process_proposal(primary, request).map_err(error_into_exception);
        }
        let block = (request.height, request.round, request.hash.clone());
        let from_prepared = handler::answers_from_the_prepared_proposal(primary, &request);
        let began = Instant::now();
        let (answer, fees) = recorded(&primary.platform().drive, || {
            handler::process_proposal(primary, request)
        });
        let answer = answer.map_err(error_into_exception)?;
        if answer.status != ProposalStatus::Accept as i32 {
            return Ok(answer);
        }
        let mut side = self.primary_side();
        if !from_prepared {
            side.milliseconds = milliseconds(began);
        }
        let executed = side.executed(block, answer.clone(), fees, from_prepared);
        side.executed = Some(executed);
        Ok(answer)
    }

    /// `FinalizeBlock`: the block is applied to both databases and compared, in the order
    /// the module says. The answer is the primary's.
    pub fn finalize_block<A>(
        &self,
        primary: &A,
        request: proto::RequestFinalizeBlock,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException>
    where
        A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    {
        let height = u64::try_from(request.height).unwrap_or_default();
        if let Some(ended_at) = self.latches.ended() {
            return Err(self.ended_exception(ended_at));
        }
        // One look at the latches for the whole block: whether the shadow applies it, and
        // a stop another thread made, which is written down with this height, the first
        // the shadow does not apply.
        let (applies, stop_to_record) = self.latches.at_a_block();
        if let Some(reason) = stop_to_record {
            self.write_the_stop(height, reason);
        }
        let Some(app) = self.app.as_ref().filter(|_| applies) else {
            let answer = handler::finalize_block(primary, request).map_err(error_into_exception)?;
            self.committed(height, false, None, Outcome::WithoutShadow);
            return Ok(answer);
        };
        let (shadow_height, shadow_holds_a_block) = held_by(&app.platform.state.load());
        if shadow_holds_a_block.is_some() && height <= shadow_height {
            return self.skip(primary, app, request, height);
        }

        #[cfg(any(test, feature = "mocks"))]
        testing::planted_before_the_block(self, primary);

        // A request that no block is made from is the primary's to refuse: the shadow is
        // not given it, for its failure at it would be no engine's.
        let Ok(proposal) = handler::process_proposal_request(&request) else {
            let answer = handler::finalize_block(primary, request).map_err(error_into_exception)?;
            let reason = "the request held no block for the shadow to execute, and the primary \
                          finalized it"
                .to_string();
            self.committed(height, false, None, Outcome::NotCompared(reason));
            return Ok(answer);
        };

        // The primary's execution of the block: the one it answered `ProcessProposal`
        // with, or the one it makes now where what it holds is not this block's.
        let from_prepared = handler::answers_from_the_prepared_proposal(primary, &proposal);
        let began = Instant::now();
        let (again, fees) = recorded(&primary.platform().drive, || {
            handler::execute_block_unless_current(primary, &request)
        });
        let again = again.map_err(error_into_exception)?;
        let (of_primary, primary_milliseconds) = {
            let mut side = self.primary_side();
            if let Some(answer) = again {
                if !from_prepared {
                    side.milliseconds = milliseconds(began);
                }
                let block = (request.height, request.round, request.hash.clone());
                let executed = side.executed(block, answer, fees, from_prepared);
                side.executed = Some(executed);
            }
            side.prepared = None;
            (
                side.executed
                    .take()
                    .filter(|executed| executed.is_of(&request)),
                side.milliseconds,
            )
        };

        let primary_line = of_primary.as_ref().map(|executed| {
            BlockLine::executed(&request, &executed.answer, executed.fees.as_deref())
        });

        // 1. The shadow executes the block. An answer of it that does not accept the
        //    block is a failure of the shadow, and is kept as it is. The errors Dash Core
        //    answers it with are noted from here on.
        app.platform.core_rpc.clear_errors();
        let shadow_began = Instant::now();
        let (of_shadow, shadow_fees) = recorded(&app.platform.drive, || {
            guarded(|| handler::answer_unless_current(app, &request))
        });
        let of_shadow = match of_shadow {
            Ok(Some(answer)) if answer.status == ProposalStatus::Accept as i32 => Ok(answer),
            Ok(Some(answer)) => Err((
                Some(answer),
                "the shadow does not accept the block".to_string(),
            )),
            Ok(None) => Err((
                None,
                "the shadow held an execution of the block already".to_string(),
            )),
            Err(error) => Err((None, error)),
        };
        let of_shadow = match of_shadow {
            Ok(answer) => answer,
            Err((not_accepting, error)) => {
                let step = "the execution of the block";
                // What the shadow made before it failed is kept: its answer, where it gave
                // one, and its fee calculations.
                let evidence = Evidence {
                    request: Some(&request),
                    primary: of_primary.as_ref(),
                    shadow_answer: not_accepting.as_ref(),
                    shadow_fees: Some(&shadow_fees),
                    ..Default::default()
                };
                let differences = Self::shadow_error(height, step, &error);
                let found = self.evidence_kept(
                    primary,
                    height,
                    DivergenceKind::ShadowError,
                    &differences,
                    evidence,
                );
                let stop = Stopped {
                    height,
                    reason: format!("the shadow failed at {step}: {error}"),
                };
                return self.after_a_failure_of_the_shadow(
                    primary,
                    app,
                    request,
                    found,
                    primary_line.as_ref(),
                    false,
                    stop,
                );
            }
        };

        #[cfg(any(test, feature = "mocks"))]
        let (of_shadow, shadow_fees) =
            testing::planted_in_the_execution(self, of_shadow, shadow_fees);

        // 2. The two executions, before either database commits.
        let shadow_line = BlockLine::executed(&request, &of_shadow, Some(&shadow_fees));
        let mut cost_alone_differs = false;
        if let (Some(executed), Some(line)) = (&of_primary, &primary_line) {
            let differences = compare::differences(line, &shadow_line);
            if let Some(kind) = compare::kind_of(&differences) {
                let evidence = Evidence {
                    request: Some(&request),
                    primary: Some(executed),
                    shadow_answer: Some(&of_shadow),
                    shadow_fees: Some(&shadow_fees),
                    lines: Some((line, &shadow_line)),
                    notes: executed
                        .fees
                        .as_ref()
                        .and_then(|fees| compare::first_fee_difference(fees, &shadow_fees))
                        .into_iter()
                        .collect(),
                    ..Default::default()
                };
                let found = self.evidence_kept(primary, height, kind, &differences, evidence);
                if self.on_divergence == OnDivergence::Stop {
                    self.written(
                        self.report
                            .change(|counts| found.counted_in(counts, height)),
                    );
                    return Err(self.end(height));
                }
                if kind != DivergenceKind::Cost {
                    // The shadow applies no more blocks, and the primary finalizes this
                    // one: the divergence, the block and the stop are one write. Where
                    // the primary does not commit the block, the divergence and the stop
                    // are written without it.
                    self.retire(app);
                    let stop = Stopped {
                        height,
                        reason: "the executions of the block differ".to_string(),
                    };
                    tracing::error!(
                        height,
                        reason = stop.reason,
                        "the shadow applies no more blocks"
                    );
                    return match handler::finalize_block(primary, request) {
                        Ok(answer) => {
                            let with = With {
                                stop: Some(stop),
                                found: Some(&found),
                            };
                            self.count(height, false, Some(line), Outcome::Diverged, with);
                            Ok(answer)
                        }
                        Err(error) => {
                            self.written(self.report.change(|counts| {
                                found.counted_in(counts, height);
                                counts.shadow_stop_pending = None;
                                counts.shadow_stopped.get_or_insert(stop);
                            }));
                            Err(error_into_exception(error))
                        }
                    };
                }
                self.written(
                    self.report
                        .change(|counts| found.counted_in(counts, height)),
                );
                cost_alone_differs = true;
            }
        }

        // 3. The shadow commits, and its database is flushed.
        let committed = guarded(|| {
            #[cfg(any(test, feature = "mocks"))]
            testing::planted_at_the_commit(self)?;
            let answer = handler::finalize_block(app, request.clone())?;
            #[cfg(any(test, feature = "mocks"))]
            testing::planted_after_the_commit(self)?;
            Ok(answer)
        });
        let flushed = match committed {
            Err(error) => Err((None, "the commit of the block", error)),
            Ok(answer) => {
                let flushed = guarded(|| {
                    #[cfg(any(test, feature = "mocks"))]
                    testing::planted_at_the_flush(self)?;
                    Ok(app
                        .platform
                        .drive
                        .grove
                        .flush()
                        .map_err(drive::error::Error::from)?)
                });
                match flushed {
                    Ok(()) => Ok(answer),
                    Err(error) => Err((
                        Some(answer),
                        "the flush after its commit of the block",
                        error,
                    )),
                }
            }
        };
        let shadow_final = match flushed {
            Ok(answer) => answer,
            Err((shadow_final, step, error)) => {
                // Whether the shadow's database holds the block. It does where the
                // handler answered. Where the handler failed it can have failed after
                // its commit: the database holds the block where its root is the app
                // hash the shadow's execution of the block gave.
                let shadow_holds_it = match &shadow_final {
                    Some(_) => Some(true),
                    None => contained(|| root_of(app.platform))
                        .and_then(|root| root)
                        .ok()
                        .map(|root| root == of_shadow.app_hash),
                };
                let holds = match shadow_holds_it {
                    Some(true) => "the shadow's database holds the block",
                    Some(false) => "the shadow's database does not hold the block",
                    None => {
                        "whether the shadow's database holds the block is not known: its \
                         root was not read"
                    }
                };
                let evidence = Evidence {
                    request: Some(&request),
                    primary: of_primary.as_ref(),
                    shadow_answer: Some(&of_shadow),
                    shadow_fees: Some(&shadow_fees),
                    shadow_final: shadow_final.as_ref(),
                    notes: vec![holds.to_string()],
                    ..Default::default()
                };
                let differences = Self::shadow_error(height, step, &error);
                let found = self.evidence_kept(
                    primary,
                    height,
                    DivergenceKind::ShadowError,
                    &differences,
                    evidence,
                );
                let shadow_holds_it = shadow_holds_it == Some(true);
                if shadow_holds_it {
                    self.shadow_holds(height);
                }
                let stop = Stopped {
                    height: height + u64::from(shadow_holds_it),
                    reason: format!("the shadow failed at {step}: {error}; {holds}"),
                };
                return self.after_a_failure_of_the_shadow(
                    primary,
                    app,
                    request,
                    found,
                    primary_line.as_ref(),
                    shadow_holds_it,
                    stop,
                );
            }
        };
        let shadow_milliseconds = milliseconds(shadow_began);
        self.shadow_holds(height);
        #[cfg(any(test, feature = "mocks"))]
        let shadow_final = testing::planted_between_the_commits(self, shadow_final)?;

        // 4. The primary finalizes the block. Its answer is the node's.
        let primary_began = Instant::now();
        let answer =
            handler::finalize_block(primary, request.clone()).map_err(error_into_exception)?;
        let primary_milliseconds = primary_milliseconds + milliseconds(primary_began);

        // 5. The answers to `FinalizeBlock`, and the two roots. Whatever is found from
        //    here on is found with the block committed by both and answered. The roots
        //    are read for the comparison alone, and a panic of either read is caught.
        let primary_root = contained(|| root_of(primary.platform())).and_then(|root| root);
        #[cfg(any(test, feature = "mocks"))]
        let primary_root = testing::planted_in_the_primarys_root(self, primary_root);
        let shadow_root = contained(|| root_of(app.platform)).and_then(|root| root);
        #[cfg(any(test, feature = "mocks"))]
        let shadow_root = testing::planted_in_the_shadows_root(self, shadow_root);
        let shadow_line = shadow_line.committed(&shadow_final, shadow_root.as_deref().ok());
        let primary_app_hash = hex::encode(
            primary
                .platform()
                .state
                .load()
                .last_committed_block_app_hash()
                .unwrap_or_default(),
        );
        let primary_line =
            primary_line.map(|line| line.committed(&answer, primary_root.as_deref().ok()));
        // The answers are compared whatever became of the roots, and the roots where
        // both were read. A root of the shadow that was not read is a failure of the
        // shadow; one of the primary that was not read is no difference.
        let mut differences = compare::after_the_commits(
            height,
            (
                &answer,
                primary_root.as_deref().ok(),
                primary_app_hash.as_str(),
            ),
            (
                &shadow_final,
                shadow_root.as_deref().ok(),
                shadow_line.app_hash.as_str(),
            ),
        );
        if let Err(error) = &shadow_root {
            differences.extend(Self::shadow_error(
                height,
                "the root after the commit",
                error,
            ));
        }
        if let Some(kind) = compare::kind_of(&differences) {
            let evidence = Evidence {
                request: Some(&request),
                primary: of_primary.as_ref(),
                shadow_answer: Some(&of_shadow),
                shadow_fees: Some(&shadow_fees),
                lines: primary_line.as_ref().map(|line| (line, &shadow_line)),
                primary_final: Some(&answer),
                shadow_final: Some(&shadow_final),
                notes: vec![
                    "found after both databases committed the block: the kept stores are the \
                     databases after it"
                        .to_string(),
                ],
                ..Default::default()
            };
            let found = self.evidence_kept(primary, height, kind, &differences, evidence);
            let reason = match &shadow_root {
                Err(error) => format!("the shadow failed at the root after the commit: {error}"),
                Ok(_) => "the two databases differ after the commit".to_string(),
            };
            // The divergence, the block and the stop are one write.
            let with = With {
                stop: Some(self.retired_after(app, height, reason)),
                found: Some(&found),
            };
            self.count(height, true, primary_line.as_ref(), Outcome::Diverged, with);
            self.ended_after(height);
            return Ok(answer);
        }

        // What was not compared, each by itself.
        let mut not_compared = Vec::new();
        match &of_primary {
            None => {
                not_compared.push("the two executions, for the primary's was not held".to_string())
            }
            Some(executed) if executed.fees.is_none() => not_compared.push(
                "the fee calculations, for those of the proposal this node prepared were not \
                 held"
                    .to_string(),
            ),
            Some(_) => (),
        }
        if let Err(error) = &primary_root {
            not_compared.push(format!(
                "the roots after the commit, for the primary's was not read ({error})"
            ));
        }
        let outcome = if cost_alone_differs {
            Outcome::Diverged
        } else if not_compared.is_empty() {
            Outcome::Compared
        } else {
            Outcome::NotCompared(format!(
                "not compared: {}; everything else was",
                not_compared.join("; ")
            ))
        };
        self.committed(height, true, primary_line.as_ref(), outcome);
        // The times of a block whose execution by the primary was held: else the
        // primary's is that of another proposal.
        if of_primary.is_some() {
            self.written(self.report.change(|counts| {
                counts.last_primary_ms = primary_milliseconds;
                counts.last_shadow_ms = shadow_milliseconds;
            }));
        }
        #[cfg(feature = "shadow-store-compare")]
        if let Err(panic) = contained(|| self.compare_the_stores_when_due(primary, app, height)) {
            tracing::error!(
                height,
                panic,
                "a comparison of the stored records did not begin"
            );
            self.written(self.report.change(|counts| {
                counts.store_comparisons.failed += 1;
                counts.store_comparisons.running = false;
            }));
        }
        Ok(answer)
    }

    /// A block at a height the shadow holds already, which Tenderdash gives the primary
    /// again after a start: the primary finalizes it, and at the shadow's own height the
    /// app hash it committed is held against the shadow's.
    fn skip<A>(
        &self,
        primary: &A,
        app: &ShadowApplication<'a, C>,
        request: proto::RequestFinalizeBlock,
        height: u64,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException>
    where
        A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    {
        // What the primary answered `ProcessProposal` with for this block, where it did,
        // is kept with a difference.
        let of_primary = {
            let mut side = self.primary_side();
            side.prepared = None;
            side.executed
                .take()
                .filter(|executed| executed.is_of(&request))
        };
        let answer =
            handler::finalize_block(primary, request.clone()).map_err(error_into_exception)?;
        let (shadow_height, of_shadow) = held_by(&app.platform.state.load());
        let app_hash_compared = height == shadow_height;
        let (_, of_primary_state) = held_by(&primary.platform().state.load());
        let mut found = None;
        let mut stop = None;
        if app_hash_compared && of_primary_state != of_shadow {
            let differences = vec![Difference {
                height,
                field: "app_hash",
                this: format!("\"{}\"", hex::encode(of_primary_state.unwrap_or_default())),
                other: format!("\"{}\"", hex::encode(of_shadow.unwrap_or_default())),
            }];
            let evidence = Evidence {
                request: Some(&request),
                primary: of_primary.as_ref(),
                primary_final: Some(&answer),
                notes: vec![
                    "found at a height the shadow held before the primary: the kept stores \
                     are the databases after the block"
                        .to_string(),
                ],
                ..Default::default()
            };
            found = Some(self.evidence_kept(
                primary,
                height,
                DivergenceKind::State,
                &differences,
                evidence,
            ));
            stop = Some(self.retired_after(
                app,
                height,
                "the app hashes differ at the shadow's height".to_string(),
            ));
        }
        let diverged = found.is_some();
        let with = With {
            stop,
            found: found.as_ref(),
        };
        self.count(
            height,
            true,
            None,
            Outcome::Skipped { app_hash_compared },
            with,
        );
        if diverged {
            self.ended_after(height);
        }
        Ok(answer)
    }
}

#[cfg(any(test, feature = "mocks"))]
impl<C> Drop for Shadow<'_, C> {
    fn drop(&mut self) {
        testing::append_counts(&self.report.counts());
    }
}
