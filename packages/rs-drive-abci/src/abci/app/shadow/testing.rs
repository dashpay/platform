//! Shadow mode in tests: a shadow for an application that a test makes, with no
//! configuration of its own.

use super::{open_shadow_platform_at, Shadow, ShadowEngine, ShadowPlatform};
use crate::abci::app::BlockExecutionApplication;
use crate::config::{OnDivergence, PlatformConfig, ShadowConfig};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0MutableGetters;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Setters;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use drive::fees::record::{FeeCalculation, RecordedOperation};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use tenderdash_abci::proto::abci as proto;

/// What a test plants in the next block that both engines execute, in what the comparison
/// is given of the shadow or in what the shadow does. Each is planted once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The shadow's answer to `InitChain` is another.
    ShadowInitChainAnswer,
    /// The cost of an operation of a fee calculation of the shadow is another.
    ShadowCost,
    /// The fee of a transaction in the shadow's answer is another.
    ShadowTransactionFee,
    /// The code of a transaction in the shadow's answer is another.
    ShadowTransactionCode,
    /// The shadow's answer to `ProcessProposal` is another outside the app hash and the
    /// transactions' codes and fees.
    ShadowRestOfTheAnswer,
    /// The shadow returns an error where it would flush its database after its commit.
    ShadowFailsAtItsFlush,
    /// The shadow commits the block and returns an error after it, as a handler that
    /// fails in what it does after its commit.
    ShadowFailsAfterItsCommit,
    /// GroveDB's root of the primary after the commit is not read.
    PrimaryRootIsNotRead,
    /// Dash Core answers a question of the shadow with an error, and the shadow fails
    /// where it would commit the block.
    CoreFailsTheShadow,
    /// The shadow returns an error where it would commit the block.
    ShadowFailsAtItsCommit,
    /// The shadow panics where it would commit the block.
    ShadowPanicsAtItsCommit,
    /// The shadow's answer to `FinalizeBlock` is another.
    ShadowFinalizeAnswer,
    /// GroveDB's root of the shadow after the commit is another.
    ShadowRootAfterTheCommit,
    /// GroveDB's root of the shadow after the commit is not read.
    ShadowRootIsNotRead,
    /// The request is answered with an error after the shadow committed the block and
    /// before the primary does, as where the node stops there.
    NodeStopsBetweenTheCommits,
    /// The node was not given `ProcessProposal` for the proposal it prepared, as far as
    /// the primary and the shadow of the application know at `FinalizeBlock`: the
    /// primary's context of the block is put back to what `PrepareProposal` left, with no
    /// block hash, and the execution that was held of `ProcessProposal` is forgotten.
    ProcessProposalIsNotGiven,
}

/// What a test planted in what the primary holds of the block before `FinalizeBlock`
/// looks at it.
pub(super) fn planted_before_the_block<A, C>(shadow: &Shadow<'_, C>, primary: &A)
where
    A: BlockExecutionApplication,
    C: CoreRPCLike,
{
    if !shadow.planted(Fault::ProcessProposalIsNotGiven) {
        return;
    }
    let mut context = primary
        .block_execution_context()
        .write()
        .expect("the context of the primary");
    context
        .as_mut()
        .expect("a context of the block")
        .block_state_info_mut()
        .set_block_hash(None);
    shadow.primary_side().executed = None;
}

/// A panic that is caught the way every panic of the shadow is, as text: for a test of
/// what the panic hook of the node does with one.
pub fn a_panic_of_the_shadow() -> String {
    super::guard::contained(|| panic!("a panic of the shadow, planted by a test"))
        .expect_err("a panic")
}

fn another_cost(fees: &mut Vec<FeeCalculation>) {
    let cost = fees.iter_mut().find_map(|calculation| {
        calculation
            .operations
            .iter_mut()
            .flatten()
            .find_map(|operation| match operation {
                RecordedOperation::Cost(cost) => Some(cost),
                _ => None,
            })
    });
    match cost {
        Some(cost) => cost.seek_count += 1,
        None => fees.push(FeeCalculation {
            transaction: None,
            base_operations: None,
            operations: Some(vec![RecordedOperation::Cost(Default::default())]),
            reads: vec![],
            epoch: 0,
            outcome: Ok(Default::default()),
        }),
    }
}

/// The first transaction's result in `answer`, which is given one where it has none.
fn first_result(answer: &mut proto::ResponseProcessProposal) -> &mut proto::ExecTxResult {
    if answer.tx_results.is_empty() {
        answer.tx_results.push(Default::default());
    }
    &mut answer.tx_results[0]
}

/// What a test planted in what the comparison is given of the shadow's execution.
pub(super) fn planted_in_the_execution<C: CoreRPCLike>(
    shadow: &Shadow<'_, C>,
    mut answer: proto::ResponseProcessProposal,
    mut fees: Vec<FeeCalculation>,
) -> (proto::ResponseProcessProposal, Vec<FeeCalculation>) {
    if shadow.planted(Fault::ShadowCost) {
        another_cost(&mut fees);
    }
    if shadow.planted(Fault::ShadowTransactionFee) {
        first_result(&mut answer).gas_used += 1;
    }
    if shadow.planted(Fault::ShadowTransactionCode) {
        first_result(&mut answer).code += 1;
    }
    if shadow.planted(Fault::ShadowRestOfTheAnswer) {
        first_result(&mut answer).info.push_str("planted by a test");
    }
    (answer, fees)
}

/// What a test planted where the shadow would commit the block.
pub(super) fn planted_at_the_commit<C: CoreRPCLike>(shadow: &Shadow<'_, C>) -> Result<(), Error> {
    if shadow.planted(Fault::ShadowFailsAtItsCommit) {
        return Err(planted_error("planted by a test"));
    }
    if shadow.planted(Fault::CoreFailsTheShadow) {
        if let Some(app) = &shadow.app {
            app.platform
                .core_rpc
                .note("get_transaction: planted by a test");
        }
        return Err(planted_error("planted by a test, after an error of Core"));
    }
    if shadow.planted(Fault::ShadowPanicsAtItsCommit) {
        panic!("planted by a test");
    }
    Ok(())
}

/// What a test planted between the commit of the shadow and the primary's.
pub(super) fn planted_between_the_commits<C: CoreRPCLike>(
    shadow: &Shadow<'_, C>,
    mut answer: proto::ResponseFinalizeBlock,
) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException> {
    if shadow.planted(Fault::NodeStopsBetweenTheCommits) {
        return Err(proto::ResponseException {
            error: "planted by a test: the node stops between the two commits".to_string(),
        });
    }
    if shadow.planted(Fault::ShadowFinalizeAnswer) {
        answer.retain_height += 1;
    }
    Ok(answer)
}

fn planted_error(what: &'static str) -> Error {
    Error::Execution(ExecutionError::CorruptedCodeExecution(what))
}

/// What a test planted where the shadow's handler has committed the block.
pub(super) fn planted_after_the_commit<C: CoreRPCLike>(
    shadow: &Shadow<'_, C>,
) -> Result<(), Error> {
    match shadow.planted(Fault::ShadowFailsAfterItsCommit) {
        true => Err(planted_error("planted by a test, after the commit")),
        false => Ok(()),
    }
}

/// What a test planted where the shadow's database would be flushed.
pub(super) fn planted_at_the_flush<C: CoreRPCLike>(shadow: &Shadow<'_, C>) -> Result<(), Error> {
    match shadow.planted(Fault::ShadowFailsAtItsFlush) {
        true => Err(planted_error("planted by a test, at the flush")),
        false => Ok(()),
    }
}

/// What a test planted in the primary's root after the commits.
pub(super) fn planted_in_the_primarys_root<C: CoreRPCLike>(
    shadow: &Shadow<'_, C>,
    root: Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    match shadow.planted(Fault::PrimaryRootIsNotRead) {
        true => Err("planted by a test".to_string()),
        false => root,
    }
}

/// What a test planted in the shadow's root after the commits.
pub(super) fn planted_in_the_shadows_root<C: CoreRPCLike>(
    shadow: &Shadow<'_, C>,
    root: Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    if shadow.planted(Fault::ShadowRootIsNotRead) {
        return Err("planted by a test".to_string());
    }
    root.map(|mut root| {
        if shadow.planted(Fault::ShadowRootAfterTheCommit) {
            root[0] ^= 1;
        }
        root
    })
}

#[derive(Default)]
struct GateState {
    open: bool,
    dismissed: bool,
    done: bool,
}

/// Where a test holds a comparison of stored records at its start, and learns that it
/// ended.
#[derive(Default)]
pub struct Gate {
    state: Mutex<GateState>,
    changed: Condvar,
}

impl Gate {
    fn state(&self) -> std::sync::MutexGuard<'_, GateState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn wait_until(
        &self,
        reached: impl Fn(&GateState) -> bool,
    ) -> std::sync::MutexGuard<'_, GateState> {
        let mut state = self.state();
        while !reached(&state) {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        state
    }

    /// Lets the comparison that waits at the gate, or the next one, go on.
    pub fn open(&self) {
        self.state().open = true;
        self.changed.notify_all();
    }

    /// Ends the thread of the comparison that waits at the gate with nothing compared and
    /// nothing written: the comparison of a node that stopped while it ran.
    pub fn dismiss(&self) {
        self.state().dismissed = true;
        self.changed.notify_all();
    }

    /// Waits until the thread of the comparison that waited at the gate has ended: it
    /// wrote what it found, or was dismissed.
    pub fn wait_until_done(&self) {
        drop(self.wait_until(|state| state.done));
    }

    /// Waits at the gate. Whether the comparison goes on: not where it is dismissed.
    pub(super) fn wait_until_open(&self) -> bool {
        self.wait_until(|state| state.open || state.dismissed).open
    }

    pub(super) fn done(&self) {
        self.state().done = true;
        self.changed.notify_all();
    }
}

/// The shadow a test asks for the chains it makes on its thread.
#[derive(Clone, Copy, Debug)]
pub struct ForATest {
    /// The engine of the shadow.
    pub engine: ShadowEngine,
    /// What a divergence does.
    pub on_divergence: OnDivergence,
    /// The seconds between two comparisons of the stored records; 0 makes none by the
    /// clock.
    pub store_compare_interval_s: u64,
    /// A change to the configuration of the shadow's Platform, where the test wants the
    /// shadow to be configured otherwise than the primary.
    pub alter: Option<fn(&mut PlatformConfig)>,
    /// Whether the end of the node at a divergence is a panic with the differences. Where
    /// it is not, the end does nothing but what shadow mode does itself: a `FinalizeBlock`
    /// that begins after it is answered with the error.
    pub the_end_panics: bool,
    /// A fault that is planted when the shadow is made, for one in `InitChain`.
    pub plant: Option<Fault>,
}

impl ForATest {
    /// A shadow on `engine` that lets the primary go on at a divergence.
    pub fn on(engine: ShadowEngine) -> Self {
        Self {
            engine,
            on_divergence: OnDivergence::Continue,
            store_compare_interval_s: 0,
            alter: None,
            the_end_panics: false,
            plant: None,
        }
    }
}

thread_local! {
    /// What a test said of the chains of this thread: nothing, that they have no shadow,
    /// or the shadow they have.
    static FOR_THIS_THREAD: RefCell<Option<Option<ForATest>>> = const { RefCell::new(None) };
}

/// Gives the chains that tests make on this thread from now on the shadow `asked`, which
/// goes before what the environment asks for. With `None` they have what the environment
/// asks for again.
pub fn shadow_for_the_chains_of_this_thread(asked: Option<ForATest>) {
    FOR_THIS_THREAD.with(|for_this_thread| *for_this_thread.borrow_mut() = asked.map(Some));
}

/// Gives the chains that tests make on this thread from now on no shadow, whatever the
/// environment asks for: a chain that another is held against as a node alone. It holds
/// until [`shadow_for_the_chains_of_this_thread`] is called.
pub fn no_shadow_for_the_chains_of_this_thread() {
    FOR_THIS_THREAD.with(|for_this_thread| *for_this_thread.borrow_mut() = Some(None));
}

/// The environment variable that gives the applications of the strategy tests a shadow:
/// `other` for one on the other storage engine, `same` for one on the engine of the
/// primary, which is the control.
///
/// A run with it is made with a process for each test, as `cargo nextest` makes it. The
/// Platform of a shadow stays open to the end of its process ([`shadow_for_tests`]), so
/// in one process for all the strategy tests, as `cargo test` makes it, the files that
/// the shadows of the chains before hold open add up, and under a limit of 1,024 open
/// files chains fail to open their databases.
pub const SHADOW: &str = "DRIVE_TEST_SHADOW";

/// The environment variable that names a file to which every shadow of a test appends its
/// counts, as one line of JSON, when it is dropped.
pub const SHADOW_COUNTS: &str = "DRIVE_TEST_SHADOW_COUNTS";

/// The directory of the shadow of a test, in the directory of the primary's database, so
/// that it goes with it.
pub fn shadow_directory(primary_db_path: &Path) -> PathBuf {
    primary_db_path.join("shadow-test")
}

/// The directory of the report of the shadow of a test.
pub fn report_directory(primary_db_path: &Path) -> PathBuf {
    shadow_directory(primary_db_path).join("report")
}

fn differences_kept(report: &Path) -> String {
    let mut text = String::new();
    let Ok(kept) = std::fs::read_dir(report.join("divergences")) else {
        return text;
    };
    for directory in kept.flatten() {
        if let Ok(differs) = std::fs::read_to_string(directory.path().join("differs.txt")) {
            text.push_str(&differs);
        }
    }
    text
}

/// The directory of the shadow of the `chain`th chain a test makes on one Platform: the
/// first is [`shadow_directory`], the ones after it have their number added.
fn shadow_directory_of_chain(primary_db_path: &Path, chain: usize) -> PathBuf {
    match chain {
        1 => shadow_directory(primary_db_path),
        chain => primary_db_path.join(format!("shadow-test-{chain}")),
    }
}

/// A shadow for the application of `primary`, as `asked`. Where `the_end_panics`, the
/// end of the node at a divergence is a panic with the differences.
///
/// The Platform of the shadow is never dropped, and stays open to the end of the process:
/// it has to outlive the application, which a test moves about and which borrows it. A
/// test that wants a second application on the same two databases takes the Platform from
/// the first shadow and gives it to [`shadow_on`].
///
/// A test that makes a second chain on one Platform, as one whose first chain failed at
/// its start, is given a second shadow with a database of its own, in a directory of its
/// own. It starts where the primary holds no block, and panics where it holds one: its
/// database is behind, and a chain is not left without the shadow that was asked for.
pub fn shadow_for_tests<C: CoreRPCLike>(primary: &Platform<C>, asked: ForATest) -> Shadow<'_, C> {
    let chain = (1..)
        .find(|chain| !shadow_directory_of_chain(&primary.config.db_path, *chain).exists())
        .expect("a number");
    let directory = shadow_directory_of_chain(&primary.config.db_path, chain);
    let platform =
        open_shadow_platform_at(primary, directory.join("db"), asked.engine, asked.alter)
            .expect("the Platform of the shadow of a test");
    shadow_in(
        &directory,
        primary,
        Some(Box::leak(Box::new(platform))),
        asked,
    )
    .expect("the shadow of a test")
}

/// A shadow for the application of `primary` on a Platform of the shadow that is open
/// already: the one of an application that a test dropped, as a node that starts again
/// has a new application on the databases it had.
///
/// With no Platform, the shadow is the one of a node whose shadow stopped in an earlier
/// run and whose database is not opened.
pub fn shadow_on<'a, C: CoreRPCLike>(
    primary: &'a Platform<C>,
    platform: Option<&'a ShadowPlatform<'a, C>>,
    asked: ForATest,
) -> Result<Shadow<'a, C>, super::ShadowError> {
    shadow_in(
        &shadow_directory(&primary.config.db_path),
        primary,
        platform,
        asked,
    )
}

/// The same with the directory of the shadow given.
fn shadow_in<'a, C: CoreRPCLike>(
    directory: &Path,
    primary: &'a Platform<C>,
    platform: Option<&'a ShadowPlatform<'a, C>>,
    asked: ForATest,
) -> Result<Shadow<'a, C>, super::ShadowError> {
    let report = directory.join("report");
    let settings = ShadowConfig {
        shadow_db_path: Some(directory.join("db")),
        shadow_report_path: Some(report.clone()),
        shadow_on_divergence: asked.on_divergence,
        shadow_store_compare_interval_s: asked.store_compare_interval_s,
    };
    let ends: Arc<dyn Fn() + Send + Sync> = match asked.the_end_panics {
        true => Arc::new(move || {
            panic!(
                "shadow mode ended the node at a divergence:\n{}",
                differences_kept(&report)
            )
        }),
        false => Arc::new(|| ()),
    };
    let shadow = Shadow::with_settings(primary, platform, &settings, ends)?;
    if let Some(fault) = asked.plant {
        shadow.plant(fault);
    }
    Ok(shadow)
}

/// The shadow for a chain that a test makes: the one the test asked for on this thread
/// ([`shadow_for_the_chains_of_this_thread`]), or else the one the environment asks for
/// ([`SHADOW`]), which ends the test at a divergence. `None` where neither asks for one,
/// or where the test said that its chains have none.
pub fn shadow_for_a_chain<C: CoreRPCLike>(primary: &Platform<C>) -> Option<Shadow<'_, C>> {
    if let Some(said) = FOR_THIS_THREAD.with(|for_this_thread| *for_this_thread.borrow()) {
        return said.map(|asked| shadow_for_tests(primary, asked));
    }
    let engine = match std::env::var(SHADOW).ok()?.as_str() {
        "" => return None,
        "other" => ShadowEngine::Other,
        "same" => ShadowEngine::Same,
        asked => panic!("{SHADOW} is `other` or `same`, not `{asked}`"),
    };
    Some(shadow_for_tests(
        primary,
        ForATest {
            on_divergence: OnDivergence::Stop,
            the_end_panics: true,
            ..ForATest::on(engine)
        },
    ))
}

/// Appends the counts of a shadow to the file [`SHADOW_COUNTS`] names, where it names one.
pub(super) fn append_counts(counts: &super::Counts) {
    use std::io::Write;
    let Some(path) = std::env::var_os(SHADOW_COUNTS) else {
        return;
    };
    let mut line = serde_json::to_string(counts).expect("counts are JSON");
    line.push('\n');
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}
