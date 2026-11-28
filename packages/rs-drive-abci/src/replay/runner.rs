use crate::abci::app::FullAbciApplication;
use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::replay::chain_answer::LoggedAnswer;
use crate::replay::report::{Finalized, Proposed, ReplayChecks};
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use hex::ToHex;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tenderdash_abci::proto::abci::{
    response_process_proposal, response_verify_vote_extension, RequestExtendVote,
    RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestPrepareProposal,
    RequestProcessProposal, RequestVerifyVoteExtension,
};
use tenderdash_abci::proto::ABCI_VERSION;
use tenderdash_abci::Application;

#[derive(Debug, Clone)]
pub(super) struct ReplayItem {
    pub(super) source: ReplaySource,
    pub(super) request: LoadedRequest,
    /// What the node that wrote the log answered, for a `ProcessProposal` request
    pub(super) logged_answer: LoggedAnswer,
}

impl ReplayItem {
    pub(super) fn from_log(
        path: &Path,
        line: usize,
        timestamp: Option<String>,
        endpoint: Option<String>,
        request: LoadedRequest,
    ) -> Self {
        Self {
            source: ReplaySource {
                path: path.to_path_buf(),
                line,
                timestamp,
                endpoint,
            },
            request,
            logged_answer: LoggedAnswer::Absent,
        }
    }

    pub(super) fn with_logged_answer(mut self, logged_answer: LoggedAnswer) -> Self {
        self.logged_answer = logged_answer;
        self
    }

    pub(super) fn describe(&self) -> String {
        self.source.describe()
    }
}

#[derive(Debug, Clone)]
pub(super) struct ReplaySource {
    path: PathBuf,
    line: usize,
    timestamp: Option<String>,
    endpoint: Option<String>,
}

impl ReplaySource {
    fn describe(&self) -> String {
        let mut out = format!("{}:{}", self.path.display(), self.line);
        if let Some(endpoint) = &self.endpoint {
            write!(&mut out, " {}", endpoint).ok();
        }
        if let Some(ts) = &self.timestamp {
            write!(&mut out, " @{}", ts).ok();
        }
        out
    }

    pub(super) fn line(&self) -> usize {
        self.line
    }
}

#[derive(Debug, Clone)]
pub(super) enum LoadedRequest {
    InitChain(RequestInitChain),
    Info(RequestInfo),
    Prepare(RequestPrepareProposal),
    Process(RequestProcessProposal),
    Finalize(RequestFinalizeBlock),
    ExtendVote(RequestExtendVote),
    VerifyVoteExtension(RequestVerifyVoteExtension),
}

impl LoadedRequest {
    pub(super) fn block_height(&self) -> Option<u64> {
        fn to_height(value: i64) -> Option<u64> {
            u64::try_from(value).ok()
        }

        match self {
            LoadedRequest::InitChain(_) => Some(0),
            LoadedRequest::Info(_) => None,
            LoadedRequest::Prepare(request) => to_height(request.height),
            LoadedRequest::Process(request) => to_height(request.height),
            LoadedRequest::Finalize(request) => to_height(request.height),
            LoadedRequest::ExtendVote(request) => to_height(request.height),
            LoadedRequest::VerifyVoteExtension(request) => to_height(request.height),
        }
    }
}

pub(super) fn ensure_db_directory(path: &Path) -> Result<bool, Box<dyn Error>> {
    if path.exists() {
        if !path.is_dir() {
            return Err(format!("{} exists but is not a directory", path.display()).into());
        }
        return Ok(false);
    }

    fs::create_dir_all(path)?;
    tracing::info!(
        "database directory {} was missing; created a fresh instance (expect InitChain)",
        path.display()
    );
    Ok(true)
}

/// Whether GroveDB's root hash may differ from the app hash after the block at `height`.
///
/// On mainnet the commit of block 32326 failed with a conflict and the chain went on with the
/// app hash that block had computed. `finalize_block` reproduces that for a commit conflict at
/// the heights 32326 to 32328 and stops the node at a failed commit anywhere else, so a replay
/// lets the two differ after those three blocks and nowhere else. `info` lets them differ
/// below height 33000; that is the range in which a node may be started on such a state, and
/// not one in which a commit may fail.
///
/// The replay is not told whether a commit failed. At those three heights it asks the store
/// which block it holds, before `FinalizeBlock` and, where the root differs, after
/// (`stored_block_height`, `store_after_block`, `root_after_block`).
fn grove_root_may_differ_from_app_hash(config: &PlatformConfig, height: i64) -> bool {
    config.network == Network::Mainnet
        && config.abci.chain_id == "evo1"
        && (32326..32329).contains(&height)
}

/// What the store says of a block once its `FinalizeBlock` has returned.
#[derive(Debug, PartialEq, Eq)]
enum RootAfterBlock {
    /// GroveDB's root hash is the block's app hash
    AppHash,
    /// The root is not the app hash, and the store holds the earlier block it held before
    /// `FinalizeBlock`: the block's platform state, which is saved in the block's
    /// transaction, was not committed. That is what the commit conflict leaves from which
    /// the chain went on
    NotCommitted,
    /// The root is not the app hash, and it may not differ: at a height where the node
    /// stops at a failed commit, or with a store that holds the block, or one that does not
    /// show an earlier block it still holds
    Mismatch,
}

/// Which block the store holds once `FinalizeBlock` of the block at `height` has returned,
/// by the heights `stored_block_height` gave before the request and after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreAfterBlock {
    /// The block: its transaction was committed
    TheBlock,
    /// The same earlier block as before the request: the block's platform state was not
    /// committed. Of a store whose transactions are committed whole or not at all, nothing
    /// of the block's transaction was
    WhatItHeldBefore,
    /// Anything else: no saved state, another block than before, or a block that is not an
    /// earlier one. None of these shows an earlier block that the store still holds, also
    /// where a transaction that was not committed would leave it so (no saved state before
    /// and none after)
    SomethingElse,
}

fn store_after_block(height: u64, before: Option<u64>, after: Option<u64>) -> StoreAfterBlock {
    match after {
        Some(stored) if stored == height => StoreAfterBlock::TheBlock,
        Some(stored) if stored < height && before == after => StoreAfterBlock::WhatItHeldBefore,
        _ => StoreAfterBlock::SomethingElse,
    }
}

/// `store` is `Some` only where `grove_root_may_differ_from_app_hash` and the root does
/// differ, and `None` everywhere else: at those heights the store is read before
/// `FinalizeBlock`, and after it where the root differs. A root may differ from the app hash
/// only after a block the store shows was not committed: a block that was committed has to
/// have left its app hash for a root.
///
/// What this does not tell apart is why a block was not committed: the commit conflict
/// `finalize_block` goes on from, or a store that lost the commit otherwise and left what it
/// held. Either way the root is not the app hash in the block's line, and the replay's last
/// lines name the height, if the block passes the replay's other checks.
fn root_after_block(
    app_hash: &[u8],
    root: &[u8; 32],
    store: Option<StoreAfterBlock>,
) -> RootAfterBlock {
    if root[..] == *app_hash {
        RootAfterBlock::AppHash
    } else if store == Some(StoreAfterBlock::WhatItHeldBefore) {
        RootAfterBlock::NotCommitted
    } else {
        RootAfterBlock::Mismatch
    }
}

/// The height of the block the store holds: that of the platform state saved in it, `None`
/// where none is saved. A block saves its platform state in its own transaction, so the
/// store holds a block's state when that block's transaction was committed and goes on
/// holding the state it held when it was not, whatever was written outside the transaction
/// meanwhile.
fn stored_block_height<C>(app: &FullAbciApplication<C>) -> Result<Option<u64>, Box<dyn Error>> {
    let state = app.platform.state.load();
    let platform_version = state.current_platform_version()?;
    let stored = Platform::<C>::fetch_platform_state(&app.platform.drive, None, platform_version)?;
    Ok(stored.and_then(|stored| {
        stored
            .last_committed_block_info()
            .as_ref()
            .map(|info| info.basic_info().height)
    }))
}

/// GroveDB's root hash as it is committed.
fn committed_grove_root<C>(app: &FullAbciApplication<C>) -> Result<[u8; 32], Box<dyn Error>> {
    let state = app.platform.state.load();
    let grove_version = &state.current_platform_version()?.drive.grove_version;
    app.platform
        .drive
        .grove
        .root_hash(None, grove_version)
        .unwrap()
        .map_err(|err| format!("{:?}", err).into())
}

/// Whether a request, executed after a `ProcessProposal`, may have replaced what that
/// proposal's execution left for `FinalizeBlock` to commit. The replay takes the answer to a
/// proposal for the finalized block's only where no such request lies between the two.
///
/// Every request is taken to, but `FinalizeBlock` itself and the three whose handlers only
/// read: `info`, which is given the platform and neither the transaction nor the block
/// execution context, and `ExtendVote` and `VerifyVoteExtension`, which answer from the
/// withdrawal transactions `ProcessProposal` kept for the blocks it accepted, whichever block
/// the vote is for. Tenderdash sends the two vote requests between the proposal and the
/// finalization of every block a validator signs, and `VerifyVoteExtension` for the commit
/// of a block a node catches up with.
fn comes_between_proposal_and_finalization(request: &LoadedRequest) -> bool {
    !matches!(
        request,
        LoadedRequest::Finalize(_)
            | LoadedRequest::Info(_)
            | LoadedRequest::ExtendVote(_)
            | LoadedRequest::VerifyVoteExtension(_)
    )
}

pub(super) fn execute_request<C>(
    app: &FullAbciApplication<C>,
    item: ReplayItem,
    progress: Option<&mut ProgressReporter>,
    checks: &mut ReplayChecks,
) -> Result<Option<u64>, Box<dyn Error>>
where
    C: crate::rpc::core::CoreRPCLike,
{
    let origin = item.describe();
    let logged_answer = item.logged_answer;
    let mut committed_height = None;
    if comes_between_proposal_and_finalization(&item.request) {
        checks.request_begins();
    }
    match item.request {
        LoadedRequest::InitChain(request) => {
            tracing::debug!("executing init_chain from {}", origin);
            let response = app.init_chain(request).map_err(|err| {
                logged_error(format!("init_chain failed for {}: {:?}", origin, err))
            })?;
            tracing::info!(
                "init_chain result ({}): app_hash=0x{}, validator_updates={}",
                origin,
                hex::encode(&response.app_hash),
                response
                    .validator_set_update
                    .as_ref()
                    .map(|v| v.validator_updates.len())
                    .unwrap_or(0)
            );
            committed_height = app
                .platform
                .state
                .load()
                .last_committed_block_info()
                .as_ref()
                .map(|info| info.basic_info().height);
        }
        LoadedRequest::Info(request) if !tenderdash_abci::check_version(&request.abci_version) => {
            // The request says the ABCI version of the Tenderdash that wrote the log, and a
            // build of another ABCI version refuses it before it reads anything. `info` reads
            // the state and changes none of it, so the blocks replay the same without it.
            tracing::warn!(
                "passing over info from {}: its ABCI version {} is not this build's {}",
                origin,
                request.abci_version,
                ABCI_VERSION
            );
            checks.info_passed_over();
        }
        LoadedRequest::Info(request) => {
            tracing::debug!("executing info from {}", origin);
            let response = app
                .info(request)
                .map_err(|err| logged_error(format!("info failed for {}: {:?}", origin, err)))?;
            tracing::info!(
                "info result ({}): last_block_height={}, last_block_app_hash=0x{}",
                origin,
                response.last_block_height,
                hex::encode(response.last_block_app_hash)
            );
        }
        LoadedRequest::Prepare(request) => {
            let height = request.height;
            let context = format_height_round(Some(height), Some(request.round));
            tracing::debug!(
                "executing prepare_proposal from {} (height={})",
                origin,
                height
            );
            let response = app.prepare_proposal(request).map_err(|err| {
                logged_error(format!(
                    "prepare_proposal failed for {}{}: {:?}",
                    origin, context, err
                ))
            })?;
            tracing::debug!(
                "prepare_proposal result ({}): height={}, app_hash=0x{}, tx_results={}, tx_records={}",
                origin,
                height,
                response.app_hash.encode_hex::<String>(),
                response.tx_results.len(),
                response.tx_records.len()
            );
        }
        LoadedRequest::Process(request) => {
            let height = request.height;
            let round = request.round;
            let core_height = request.core_chain_locked_height;
            let block_hash = request.hash.clone();
            let context = format_height_round(Some(height), Some(request.round));
            tracing::debug!(
                "executing process_proposal from {} (height={})",
                origin,
                height
            );
            // What the block is held against is read before the handler of `ProcessProposal`
            // is called: the execution of a proposal may write outside the block's
            // transaction. A `PrepareProposal` of the height that came before this request
            // has been executed
            checks
                .block_begins(u64::try_from(height)?)
                .map_err(|reason| {
                    logged_error(format!(
                        "replay stops before process_proposal for {}{}:\n{}",
                        origin, context, reason
                    ))
                })?;
            let response = app.process_proposal(request).map_err(|err| {
                logged_error(format!(
                    "process_proposal failed for {}{}: {:?}",
                    origin, context, err
                ))
            })?;
            let status = response_process_proposal::ProposalStatus::try_from(response.status)
                .unwrap_or(response_process_proposal::ProposalStatus::Unknown);
            tracing::debug!(
                "process_proposal result ({}): status={:?}, height={}, app_hash=0x{}, tx_results={}, events={}",
                origin,
                status,
                height,
                hex::encode(&response.app_hash),
                response.tx_results.len(),
                response.events.len()
            );
            checks
                .proposal_answered(
                    Proposed {
                        height: u64::try_from(height)?,
                        round: u32::try_from(round)?,
                        block_hash: &block_hash,
                        core_height,
                    },
                    &response,
                    &logged_answer,
                )
                .map_err(|reason| {
                    logged_error(format!(
                        "replay stops after process_proposal for {}{}:\n{}",
                        origin, context, reason
                    ))
                })?;
        }
        LoadedRequest::Finalize(request) => {
            let height = request.height;
            let round = request.round;
            let context = format_height_round(Some(height), Some(round));
            tracing::debug!(
                "executing finalize_block from {} (height={}, round={})",
                origin,
                height,
                round
            );
            let expected = extract_expected_app_hash(&request);
            let block_hash = request.hash.clone();
            let transactions = request
                .block
                .as_ref()
                .and_then(|block| block.data.as_ref())
                .map_or(0, |data| data.txs.len());

            // Read at the block's `ProcessProposal`, and here for a block that had none
            checks
                .block_begins(u64::try_from(height)?)
                .map_err(|reason| {
                    logged_error(format!(
                        "replay stops before finalize_block for {}{}:\n{}",
                        origin, context, reason
                    ))
                })?;
            // Where the chain went on from a block that was not committed: the block the
            // store holds before the request, to hold what it holds afterwards against
            let stored_before = if grove_root_may_differ_from_app_hash(&app.platform.config, height)
            {
                Some(stored_block_height(app).map_err(|err| {
                    logged_error(format!(
                        "the platform state the store holds cannot be read before finalize_block for {}{}: {}",
                        origin, context, err
                    ))
                })?)
            } else {
                None
            };
            // After an error of `finalizing` the handler was left in the middle of the block:
            // the replay executes nothing more on this platform, and ends. The platform's own
            // end still flushes and closes GroveDB, which commits nothing of the block
            let response = checks
                .finalizing(|| app.finalize_block(request))
                .map_err(|reason| {
                    logged_error(format!(
                        "finalize_block was ended before the block's transaction was committed for {}{}: {}",
                        origin, context, reason
                    ))
                })?
                .map_err(|err| {
                    logged_error(format!(
                        "finalize_block failed for {}{}: {:?}",
                        origin, context, err
                    ))
                })?;
            let actual_hash = app
                .platform
                .state
                .load()
                .last_committed_block_app_hash()
                .map(|hash| hash.to_vec());
            let grove_hash = committed_grove_root(app).map_err(|err| {
                // A root that cannot be read is not a root that differs
                logged_error(format!(
                    "grovedb root hash cannot be read after finalize_block for {}{}: {}",
                    origin, context, err
                ))
            })?;
            let actual_hex = actual_hash
                .as_ref()
                .map(hex::encode)
                .unwrap_or_else(|| "unknown".to_string());
            tracing::debug!(
                "finalize_block result ({}): height={}, retain_height={}, state_app_hash=0x{}, grove_root=0x{}",
                origin,
                height,
                response.retain_height,
                actual_hex,
                hex::encode(grove_hash)
            );
            match (expected.as_ref(), actual_hash.as_ref()) {
                (Some(expected_hash), Some(actual_hash)) => {
                    if expected_hash != actual_hash {
                        return Err(logged_error(format!(
                            "app_hash mismatch for {}{}: expected 0x{}, got 0x{}",
                            origin,
                            context,
                            hex::encode(expected_hash),
                            actual_hex
                        )));
                    }
                    // Asked a second time only where the answer decides: the root differs at
                    // a height the chain went on from without the block's commit
                    let stored = match stored_before {
                        Some(before) if expected_hash != &grove_hash => {
                            let after = stored_block_height(app).map_err(|err| {
                                logged_error(format!(
                                    "the platform state the store holds cannot be read after finalize_block for {}{}: {}",
                                    origin, context, err
                                ))
                            })?;
                            Some((before, after))
                        }
                        _ => None,
                    };
                    let store = match stored {
                        Some((before, after)) => {
                            Some(store_after_block(u64::try_from(height)?, before, after))
                        }
                        None => None,
                    };
                    match root_after_block(expected_hash, &grove_hash, store) {
                        RootAfterBlock::AppHash => {}
                        RootAfterBlock::NotCommitted => tracing::warn!(
                            "grovedb root is not the app hash for {}{}: app hash 0x{}, grove 0x{}; the store holds the platform state of block {:?}, as before finalize_block, so this block was not committed. The chain went on from such a block here, and so does the replay",
                            origin,
                            context,
                            hex::encode(expected_hash),
                            hex::encode(grove_hash),
                            stored.and_then(|(_, after)| after)
                        ),
                        RootAfterBlock::Mismatch => {
                            let held = match (store, stored) {
                                (Some(StoreAfterBlock::TheBlock), _) => {
                                    "; the store holds this block, so it was committed with a root that is not its app hash".to_owned()
                                }
                                (Some(_), Some((before, after))) => format!(
                                    "; the store held the platform state of block {:?} before finalize_block and holds that of block {:?} now, which is not this block and does not show an earlier block that the store still holds",
                                    before, after
                                ),
                                _ => String::new(),
                            };
                            return Err(logged_error(format!(
                                "grovedb root mismatch for {}{}: expected 0x{}, grove 0x{}{}",
                                origin,
                                context,
                                hex::encode(expected_hash),
                                hex::encode(grove_hash),
                                held
                            )));
                        }
                    }
                }
                (Some(_), None) => {
                    return Err(logged_error(format!(
                        "finalize_block executed for {}{} but platform state did not expose app_hash",
                        origin, context
                    )));
                }
                (None, Some(_)) => {
                    tracing::warn!(
                        "could not extract expected app_hash from finalize_block request {}{}",
                        origin,
                        context
                    );
                }
                (None, None) => {
                    tracing::warn!(
                        "could not extract expected app_hash from request or state for {}{}",
                        origin,
                        context
                    );
                }
            }
            checks
                .block_finalized(
                    Finalized {
                        height: u64::try_from(height)?,
                        round: u32::try_from(round)?,
                        block_hash: &block_hash,
                        transactions,
                        protocol_version: app
                            .platform
                            .state
                            .load()
                            .current_protocol_version_in_consensus(),
                        app_hash: actual_hash.as_deref().unwrap_or_default(),
                        grove_root: &grove_hash,
                    },
                    &response,
                )
                .map_err(|reason| {
                    logged_error(format!(
                        "replay stops after finalize_block for {}{}:\n{}",
                        origin, context, reason
                    ))
                })?;
            if let Some(reporter) = progress {
                if let Ok(block_height) = u64::try_from(height) {
                    if let Some(hash) = actual_hash.as_deref().or(expected.as_deref()) {
                        reporter.record(block_height, hash);
                    }
                }
            }
            committed_height = u64::try_from(height).ok();
        }
        LoadedRequest::ExtendVote(request) => {
            let context = format_height_round(Some(request.height), Some(request.round));
            tracing::debug!(
                "executing extend_vote from {} (height={}, round={})",
                origin,
                request.height,
                request.round
            );
            let response = app.extend_vote(request).map_err(|err| {
                logged_error(format!(
                    "extend_vote failed for {}{}: {:?}",
                    origin, context, err
                ))
            })?;
            let total_bytes: usize = response
                .vote_extensions
                .iter()
                .map(|ext| ext.extension.len())
                .sum();
            tracing::debug!(
                "extend_vote result ({}): vote_extensions={}, total_extension_bytes={}",
                origin,
                response.vote_extensions.len(),
                total_bytes
            );
        }
        LoadedRequest::VerifyVoteExtension(request) => {
            let context = format_height_round(Some(request.height), Some(request.round));
            tracing::debug!(
                "executing verify_vote_extension from {} (height={}, round={})",
                origin,
                request.height,
                request.round
            );
            let response = app.verify_vote_extension(request).map_err(|err| {
                logged_error(format!(
                    "verify_vote_extension failed for {}{}: {:?}",
                    origin, context, err
                ))
            })?;
            let status = response_verify_vote_extension::VerifyStatus::try_from(response.status)
                .unwrap_or(response_verify_vote_extension::VerifyStatus::Unknown);
            tracing::debug!(
                "verify_vote_extension result ({}): status={:?}",
                origin,
                status
            );
        }
    }

    Ok(committed_height)
}

const PROGRESS_MIN_INTERVAL: Duration = Duration::from_secs(1);
const PROGRESS_WINDOW_SHORT: Duration = Duration::from_secs(60);
const PROGRESS_WINDOW_LONG: Duration = Duration::from_secs(300);

pub(super) struct ProgressReporter {
    history: VecDeque<(Instant, u64)>,
    last_emit: Option<Instant>,
    stop_height: Option<u64>,
}

impl ProgressReporter {
    pub(super) fn new(stop_height: Option<u64>) -> Self {
        Self {
            history: VecDeque::new(),
            last_emit: None,
            stop_height,
        }
    }

    pub(super) fn record(&mut self, height: u64, app_hash: &[u8]) {
        let now = Instant::now();
        self.history.push_back((now, height));
        self.trim_history(now);

        if let Some(last) = self.last_emit {
            if now.duration_since(last) < PROGRESS_MIN_INTERVAL {
                return;
            }
        }

        let rate_1m = self.rate_per_minute(now, PROGRESS_WINDOW_SHORT);
        let rate_5m = self.rate_per_minute(now, PROGRESS_WINDOW_LONG);
        let eta = Self::format_eta(self.eta(rate_5m, height));
        tracing::info!(
            height,
            app_hash = Self::short_hash(app_hash),
            rate_1m = Self::format_rate(rate_1m),
            rate_5m = Self::format_rate(rate_5m),
            eta,
            "block processed",
        );
        self.last_emit = Some(now);
    }

    fn trim_history(&mut self, now: Instant) {
        while let Some((ts, _)) = self.history.front() {
            if now.duration_since(*ts) > PROGRESS_WINDOW_LONG {
                self.history.pop_front();
            } else {
                break;
            }
        }
    }

    fn rate_per_minute(&self, now: Instant, window: Duration) -> Option<f64> {
        let latest = self.history.back()?;
        let start = self
            .history
            .iter()
            .find(|(ts, _)| now.duration_since(*ts) <= window)?;
        let elapsed = now.duration_since(start.0).as_secs_f64();
        if elapsed <= 0.0 {
            return None;
        }
        let delta = latest.1.saturating_sub(start.1) as f64;
        Some(delta / elapsed * 60.0)
    }

    fn format_rate(rate: Option<f64>) -> String {
        rate.map(|value| format!("{:.2} blk/min", value))
            .unwrap_or_else(|| "n/a".to_string())
    }

    fn eta(&self, rate_5m: Option<f64>, current_height: u64) -> Option<Duration> {
        let target = self.stop_height?;
        if current_height >= target {
            return Some(Duration::from_secs(0));
        }
        let rate = rate_5m?;
        if rate <= 0.0 {
            return None;
        }
        let remaining = target.saturating_sub(current_height) as f64;
        let seconds = (remaining / rate * 60.0).round();
        if seconds.is_finite() && seconds >= 0.0 {
            Some(Duration::from_secs(seconds as u64))
        } else {
            None
        }
    }

    fn format_eta(duration: Option<Duration>) -> String {
        duration
            .map(|d| {
                let total = d.as_secs();
                let hours = total / 3600;
                let minutes = (total % 3600) / 60;
                let seconds = total % 60;
                if hours > 0 {
                    format!("{:02}h{:02}m{:02}s", hours, minutes, seconds)
                } else if minutes > 0 {
                    format!("{:02}m{:02}s", minutes, seconds)
                } else {
                    format!("{:02}s", seconds)
                }
            })
            .unwrap_or_else(|| "n/a".to_string())
    }

    fn short_hash(app_hash: &[u8]) -> String {
        let hex = hex::encode(app_hash);
        let end = hex.len().min(10);
        hex[..end].to_string()
    }
}

fn logged_error(message: String) -> Box<dyn Error> {
    tracing::error!("{}", message);
    message.into()
}

pub(super) fn log_last_committed_block<C>(platform: &Platform<C>)
where
    C: crate::rpc::core::CoreRPCLike,
{
    let platform_state = platform.state.load();
    let grove_version = &platform_state
        .current_platform_version()
        .unwrap_or(PlatformVersion::latest())
        .drive
        .grove_version;
    let grove_hash = platform
        .drive
        .grove
        .root_hash(None, grove_version)
        .unwrap()
        .unwrap_or_default();

    if let Some(info) = platform_state.last_committed_block_info() {
        let app_hash = info.app_hash();
        let basic_info = info.basic_info();
        tracing::info!(
            height = basic_info.height,
            round = info.round(),
            core_height = basic_info.core_height,
            block_id_hash = %hex::encode(info.block_id_hash()),
            app_hash = %hex::encode(app_hash),
            grove_hash = %hex::encode(grove_hash),
            "Platform state at last committed block",
        );
    } else {
        tracing::info!("last_committed_block: None");
    }
}

pub(super) fn stop_height_reached(limit: Option<u64>, known_height: Option<u64>) -> bool {
    matches!((limit, known_height), (Some(limit), Some(height)) if height >= limit)
}

fn extract_expected_app_hash(request: &RequestFinalizeBlock) -> Option<Vec<u8>> {
    request
        .block
        .as_ref()
        .and_then(|block| block.header.as_ref())
        .map(|header| header.app_hash.clone())
}

fn format_height_round(height: Option<i64>, round: Option<i32>) -> String {
    let mut parts = Vec::new();
    if let Some(h) = height {
        parts.push(format!("height={}", h));
    }
    if let Some(r) = round {
        parts.push(format!("round={}", r));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::block_execution_context::v0::{
        BlockExecutionContextV0, BlockExecutionContextV0Getters,
    };
    use crate::execution::types::block_execution_context::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::{
        BlockStateInfoV0, BlockStateInfoV0Methods,
    };
    use crate::execution::types::block_state_info::BlockStateInfo;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use std::collections::BTreeMap;
    use tenderdash_abci::proto::abci::{
        ExecTxResult, RequestExtendVote, RequestFinalizeBlock, RequestInfo, RequestInitChain,
        RequestPrepareProposal, RequestProcessProposal, RequestVerifyVoteExtension,
    };
    use tenderdash_abci::proto::abci::{ResponseFinalizeBlock, ResponseProcessProposal};
    use tenderdash_abci::proto::types::{Block, Header};

    // ---- where GroveDB's root may differ from the app hash ----

    #[test]
    fn should_let_the_grove_root_differ_only_where_mainnet_went_on_with_it() {
        let mut mainnet = PlatformConfig::default_mainnet();
        mainnet.abci.chain_id = "evo1".to_owned();
        assert!(!grove_root_may_differ_from_app_hash(&mainnet, 1));
        assert!(!grove_root_may_differ_from_app_hash(&mainnet, 32325));
        // The three heights at which `finalize_block` goes on after a commit conflict
        assert!(grove_root_may_differ_from_app_hash(&mainnet, 32326));
        assert!(grove_root_may_differ_from_app_hash(&mainnet, 32327));
        assert!(grove_root_may_differ_from_app_hash(&mainnet, 32328));
        // From there on a block is committed or the node stops, also below the height up to
        // which `info` lets a node start on such a state
        assert!(!grove_root_may_differ_from_app_hash(&mainnet, 32329));
        assert!(!grove_root_may_differ_from_app_hash(&mainnet, 32999));
        assert!(!grove_root_may_differ_from_app_hash(&mainnet, 33000));
        assert!(!grove_root_may_differ_from_app_hash(&mainnet, 478_271));

        // Another chain of the same network kind, and another network with that chain's name
        let mut other_chain = PlatformConfig::default_mainnet();
        other_chain.abci.chain_id = "devnet".to_owned();
        assert!(!grove_root_may_differ_from_app_hash(&other_chain, 32326));
        let mut testnet = PlatformConfig::default_testnet();
        testnet.abci.chain_id = "evo1".to_owned();
        assert!(!grove_root_may_differ_from_app_hash(&testnet, 32326));
    }

    #[test]
    fn should_take_a_root_that_is_not_the_app_hash_only_after_a_block_that_was_not_committed() {
        let app_hash = [1u8; 32];
        let other = [3u8; 32];
        // The root is the app hash, whatever the store holds
        assert_eq!(
            root_after_block(&app_hash, &app_hash, None),
            RootAfterBlock::AppHash
        );
        // At a height the chain went on from without the commit: the store holds what it
        // held before the block
        assert_eq!(
            root_after_block(&app_hash, &other, Some(StoreAfterBlock::WhatItHeldBefore)),
            RootAfterBlock::NotCommitted
        );
        // There too, a block that was committed has to have left its app hash for a root,
        // and a store that holds neither the block nor what it held is no block that was
        // not committed
        assert_eq!(
            root_after_block(&app_hash, &other, Some(StoreAfterBlock::TheBlock)),
            RootAfterBlock::Mismatch
        );
        assert_eq!(
            root_after_block(&app_hash, &other, Some(StoreAfterBlock::SomethingElse)),
            RootAfterBlock::Mismatch
        );
        // At every other height the store is not read, and a root that is not the app hash
        // is not taken
        assert_eq!(
            root_after_block(&app_hash, &other, None),
            RootAfterBlock::Mismatch
        );
    }

    #[test]
    fn should_tell_by_the_block_the_store_holds_whether_a_block_was_committed() {
        // Block 8 was committed
        assert_eq!(
            store_after_block(8, Some(7), Some(8)),
            StoreAfterBlock::TheBlock
        );
        // It was not: the store holds what it held, the block before
        assert_eq!(
            store_after_block(8, Some(7), Some(7)),
            StoreAfterBlock::WhatItHeldBefore
        );
        // or an earlier one, where the block before was not committed either
        assert_eq!(
            store_after_block(8, Some(6), Some(6)),
            StoreAfterBlock::WhatItHeldBefore
        );
        // None of these shows an earlier block that the store still holds: the saved state
        // gone, never there, of a later block, or of another earlier block than before
        for (before, after) in [
            (Some(7), None),
            (None, None),
            (Some(7), Some(9)),
            (Some(9), Some(9)),
            (Some(7), Some(6)),
            (None, Some(7)),
        ] {
            assert_eq!(
                store_after_block(8, before, after),
                StoreAfterBlock::SomethingElse,
                "{before:?} {after:?}"
            );
        }
    }
    #[test]
    fn should_read_from_the_store_which_block_it_holds() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let app = FullAbciApplication::new(&platform.platform);
        // Nothing was saved yet
        assert_eq!(stored_block_height(&app).unwrap(), None);

        // What the commit of block 7 leaves, and what the store goes on holding while block 8
        // is executed and its transaction is not committed
        let platform_version = PlatformVersion::latest();
        let mut state = platform.platform.state.load().as_ref().clone();
        state.set_last_committed_block_info(Some(
            dpp::block::extended_block_info::v0::ExtendedBlockInfoV0 {
                basic_info: dpp::block::block_info::BlockInfo {
                    height: 7,
                    ..Default::default()
                },
                app_hash: [7; 32],
                quorum_hash: [0; 32],
                block_id_hash: [0; 32],
                proposer_pro_tx_hash: [0; 32],
                signature: [0; 96],
                round: 0,
            }
            .into(),
        ));
        platform
            .platform
            .store_platform_state(&state, None, platform_version)
            .unwrap();
        assert_eq!(stored_block_height(&app).unwrap(), Some(7));
        let transaction = platform.platform.drive.grove.start_transaction();
        let mut next = state.clone();
        next.set_last_committed_block_info(Some(
            dpp::block::extended_block_info::v0::ExtendedBlockInfoV0 {
                basic_info: dpp::block::block_info::BlockInfo {
                    height: 8,
                    ..Default::default()
                },
                app_hash: [8; 32],
                quorum_hash: [0; 32],
                block_id_hash: [0; 32],
                proposer_pro_tx_hash: [0; 32],
                signature: [0; 96],
                round: 0,
            }
            .into(),
        ));
        platform
            .platform
            .store_platform_state(&next, Some(&transaction), platform_version)
            .unwrap();
        assert_eq!(stored_block_height(&app).unwrap(), Some(7));
        drop(transaction);
        assert_eq!(stored_block_height(&app).unwrap(), Some(7));
    }

    #[test]
    fn should_pass_over_an_info_request_of_another_abci_version() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let app = FullAbciApplication::new(&platform.platform);
        let mut checks = ReplayChecks::default();
        let info = |abci_version: &str| {
            ReplayItem::from_log(
                Path::new("/test.log"),
                1,
                None,
                None,
                LoadedRequest::Info(RequestInfo {
                    version: "1.8.2".to_owned(),
                    block_version: 14,
                    p2p_version: 10,
                    abci_version: abci_version.to_owned(),
                }),
            )
        };

        // The version of a Tenderdash this build does not speak with
        let committed = execute_request(&app, info("999.999.999"), None, &mut checks).unwrap();
        assert_eq!(committed, None);
        assert_eq!(checks.summary().info_passed_over, 1);

        // This build's own version is answered as before
        let committed = execute_request(&app, info(ABCI_VERSION), None, &mut checks).unwrap();
        assert_eq!(committed, None);
        assert_eq!(checks.summary().info_passed_over, 1);
    }

    #[test]
    fn should_keep_the_answer_to_a_proposal_over_requests_that_only_read() {
        for request in [
            LoadedRequest::Info(RequestInfo::default()),
            LoadedRequest::Finalize(RequestFinalizeBlock::default()),
            LoadedRequest::ExtendVote(RequestExtendVote::default()),
            LoadedRequest::VerifyVoteExtension(RequestVerifyVoteExtension::default()),
        ] {
            assert!(
                !comes_between_proposal_and_finalization(&request),
                "{request:?}"
            );
        }
        for request in [
            LoadedRequest::InitChain(RequestInitChain::default()),
            LoadedRequest::Prepare(RequestPrepareProposal::default()),
            LoadedRequest::Process(RequestProcessProposal::default()),
        ] {
            assert!(
                comes_between_proposal_and_finalization(&request),
                "{request:?}"
            );
        }

        // An info request between a proposal and its finalization, replayed or passed over
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let app = FullAbciApplication::new(&platform.platform);
        let dir = tempfile::tempdir().unwrap();
        let report = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&report), None, None).unwrap();
        let block_hash = [7u8; 32];
        checks
            .proposal_answered(
                Proposed {
                    height: 5,
                    round: 0,
                    block_hash: &block_hash,
                    core_height: 1,
                },
                &ResponseProcessProposal {
                    status: response_process_proposal::ProposalStatus::Accept as i32,
                    ..Default::default()
                },
                &LoggedAnswer::Absent,
            )
            .unwrap();
        for abci_version in ["999.999.999", ABCI_VERSION] {
            let info = ReplayItem::from_log(
                Path::new("/test.log"),
                1,
                None,
                None,
                LoadedRequest::Info(RequestInfo {
                    abci_version: abci_version.to_owned(),
                    ..Default::default()
                }),
            );
            execute_request(&app, info, None, &mut checks).unwrap();
        }
        checks
            .block_finalized(
                Finalized {
                    height: 5,
                    round: 0,
                    block_hash: &block_hash,
                    transactions: 0,
                    protocol_version: 1,
                    app_hash: &[1; 32],
                    grove_root: &[1; 32],
                },
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        let line = fs::read_to_string(&report).unwrap();
        assert!(line.contains("\"core_height\":1,"), "{line}");
        assert!(line.contains("\"tx\":[],"), "{line}");
    }

    /// What `ProcessProposal` leaves for `FinalizeBlock` of a block it accepted.
    fn block_execution_context(
        height: u64,
        round: u32,
        block_hash: [u8; 32],
        platform: &Platform<MockCoreRPCLike>,
    ) -> BlockExecutionContext {
        let platform_version = PlatformVersion::latest();
        BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height,
                round,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: Some(block_hash),
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state: PlatformState::default_with_protocol_versions(
                platform_version.protocol_version,
                platform_version.protocol_version,
                &platform.config,
            )
            .expect("a default platform state"),
            proposer_results: None,
        })
    }

    #[test]
    fn should_keep_the_answer_to_a_proposal_over_the_votes_on_its_block() {
        let block_hash = [7u8; 32];
        let extend = || {
            LoadedRequest::ExtendVote(RequestExtendVote {
                hash: block_hash.to_vec(),
                height: 5,
                round: 0,
            })
        };
        let verify = || {
            LoadedRequest::VerifyVoteExtension(RequestVerifyVoteExtension {
                hash: block_hash.to_vec(),
                validator_pro_tx_hash: vec![1u8; 32],
                height: 5,
                round: 0,
                vote_extensions: Vec::new(),
            })
        };
        // A validator extends its own vote and verifies those of the others before the block
        // is finalized; a node that catches up verifies the extensions of the block's commit
        for votes in [vec![extend(), verify(), verify()], vec![verify()]] {
            let platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc();
            let app = FullAbciApplication::new(&platform.platform);
            // What `ProcessProposal` leaves of a block it accepted
            app.unsigned_withdrawal_txs_by_round
                .write()
                .unwrap()
                .insert(5, 0, block_hash, Vec::new());
            app.block_execution_context
                .write()
                .unwrap()
                .replace(block_execution_context(
                    5,
                    0,
                    block_hash,
                    &platform.platform,
                ));

            let dir = tempfile::tempdir().unwrap();
            let report = dir.path().join("report.jsonl");
            let mut checks = ReplayChecks::open(Some(&report), None, None).unwrap();
            checks
                .proposal_answered(
                    Proposed {
                        height: 5,
                        round: 0,
                        block_hash: &block_hash,
                        core_height: 1,
                    },
                    &ResponseProcessProposal {
                        status: response_process_proposal::ProposalStatus::Accept as i32,
                        tx_results: vec![ExecTxResult {
                            code: 0,
                            gas_used: 117,
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    &LoggedAnswer::Absent,
                )
                .unwrap();
            for (index, request) in votes.into_iter().enumerate() {
                let item =
                    ReplayItem::from_log(Path::new("/test.log"), index + 2, None, None, request);
                execute_request(&app, item, None, &mut checks).unwrap();
            }

            // The votes left what the proposal's execution left: its context, and no
            // transaction of their own
            assert!(app
                .block_execution_context
                .read()
                .unwrap()
                .as_ref()
                .expect("the context of the proposal")
                .block_state_info()
                .matches_current_block(5, 0, block_hash)
                .unwrap());
            assert!(app.transaction.read().unwrap().is_none());

            checks
                .block_finalized(
                    Finalized {
                        height: 5,
                        round: 0,
                        block_hash: &block_hash,
                        transactions: 1,
                        protocol_version: 1,
                        app_hash: &[1; 32],
                        grove_root: &[1; 32],
                    },
                    &ResponseFinalizeBlock::default(),
                )
                .unwrap();
            let line = fs::read_to_string(&report).unwrap();
            assert!(line.contains("\"core_height\":1,"), "{line}");
            assert!(line.contains("\"tx\":[[0,117]],"), "{line}");
            assert_eq!(checks.summary().blocks_without_proposal, 0);
            assert_eq!(checks.summary().transitions, 1);
        }
    }

    #[test]
    fn should_stop_before_finalize_block_where_the_other_report_has_no_line_for_the_block() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let app = FullAbciApplication::new(&platform.platform);
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other.jsonl");
        fs::write(&other, "").unwrap();
        let report = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&report), Some(&other), None).unwrap();

        let finalize = ReplayItem::from_log(
            Path::new("/test.log"),
            9,
            None,
            None,
            LoadedRequest::Finalize(RequestFinalizeBlock {
                height: 1,
                ..Default::default()
            }),
        );
        let error = execute_request(&app, finalize, None, &mut checks)
            .unwrap_err()
            .to_string();
        // The replay's own error, from before the handler is called: the handler has nothing
        // to finalize here and would have failed with an error of its own
        assert_eq!(
            error,
            format!(
                "replay stops before finalize_block for /test.log:9 (height=1, round=0):\nreport {} has no line for height 1: it ends before",
                other.display()
            )
        );
        assert_eq!(fs::read_to_string(&report).unwrap(), "");
    }

    #[test]
    fn should_read_the_other_reports_line_before_a_process_proposal_is_executed() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        // Core has no chain lock to give a `PrepareProposal`
        let mut core = MockCoreRPCLike::new();
        core.expect_get_best_chain_lock().returning(|| {
            Err(dpp::dashcore_rpc::Error::JsonRpc(
                dpp::dashcore_rpc::jsonrpc::Error::EmptyBatch,
            ))
        });
        platform.core_rpc = core;
        let app = FullAbciApplication::new(&platform.platform);
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other.jsonl");
        fs::write(&other, "").unwrap();
        let report = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&report), Some(&other), None).unwrap();

        // The other report has no line for block 1. A `PrepareProposal` is executed all the
        // same, as it is without another report: a log may end with one. The error is the
        // handler's, of a request that names no version
        let prepare = LoadedRequest::Prepare(RequestPrepareProposal {
            height: 1,
            ..Default::default()
        });
        let item = ReplayItem::from_log(Path::new("/test.log"), 8, None, None, prepare);
        let error = execute_request(&app, item, None, &mut checks)
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with("prepare_proposal failed for /test.log:8 (height=1, round=0): "),
            "{error}"
        );

        // A `ProcessProposal` is not: the replay's own error, from before the handler is
        // called
        let process = LoadedRequest::Process(RequestProcessProposal {
            height: 1,
            ..Default::default()
        });
        let item = ReplayItem::from_log(Path::new("/test.log"), 9, None, None, process);
        let error = execute_request(&app, item, None, &mut checks)
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            format!(
                "replay stops before process_proposal for /test.log:9 (height=1, round=0):\nreport {} has no line for height 1: it ends before",
                other.display()
            )
        );
        assert_eq!(fs::read_to_string(&report).unwrap(), "");
    }

    #[test]
    fn should_execute_nothing_of_a_block_the_other_report_has_no_line_for() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let app = FullAbciApplication::new(&platform.platform);
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other.jsonl");
        fs::write(&other, "").unwrap();
        let report = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&report), Some(&other), None).unwrap();

        // A height after the first block: the handler of such a request begins a transaction
        // before it looks at the request any further
        let process = LoadedRequest::Process(RequestProcessProposal {
            height: 2,
            ..Default::default()
        });
        let item = ReplayItem::from_log(Path::new("/test.log"), 9, None, None, process);
        let error = execute_request(&app, item, None, &mut checks)
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with("replay stops before process_proposal for "),
            "{error}"
        );
        // No transaction was begun, and no proposal left a context
        assert!(app.transaction.read().unwrap().is_none());
        assert!(app.block_execution_context.read().unwrap().is_none());
    }
    #[test]
    fn should_carry_the_logged_answer_with_the_request() {
        let item = ReplayItem::from_log(
            Path::new("/test.log"),
            7,
            None,
            None,
            LoadedRequest::Process(RequestProcessProposal::default()),
        );
        assert_eq!(item.logged_answer, LoggedAnswer::Absent);
        let item = item.with_logged_answer(LoggedAnswer::Unreadable("cut short".to_owned()));
        assert_eq!(
            item.logged_answer,
            LoggedAnswer::Unreadable("cut short".to_owned())
        );
        assert_eq!(item.source.line(), 7);
    }

    // ---- stop_height_reached ----

    #[test]
    fn stop_height_reached_no_limit() {
        assert!(!stop_height_reached(None, Some(100)));
    }

    #[test]
    fn stop_height_reached_no_known_height() {
        assert!(!stop_height_reached(Some(100), None));
    }

    #[test]
    fn stop_height_reached_both_none() {
        assert!(!stop_height_reached(None, None));
    }

    #[test]
    fn stop_height_reached_below_limit() {
        assert!(!stop_height_reached(Some(100), Some(99)));
    }

    #[test]
    fn stop_height_reached_at_limit() {
        assert!(stop_height_reached(Some(100), Some(100)));
    }

    #[test]
    fn stop_height_reached_above_limit() {
        assert!(stop_height_reached(Some(100), Some(101)));
    }

    // ---- format_height_round ----

    #[test]
    fn format_height_round_both_present() {
        let result = format_height_round(Some(42), Some(3));
        assert_eq!(result, " (height=42, round=3)");
    }

    #[test]
    fn format_height_round_height_only() {
        let result = format_height_round(Some(42), None);
        assert_eq!(result, " (height=42)");
    }

    #[test]
    fn format_height_round_round_only() {
        let result = format_height_round(None, Some(3));
        assert_eq!(result, " (round=3)");
    }

    #[test]
    fn format_height_round_neither() {
        let result = format_height_round(None, None);
        assert_eq!(result, "");
    }

    // ---- extract_expected_app_hash ----

    #[test]
    fn extract_expected_app_hash_with_block_and_header() {
        let request = RequestFinalizeBlock {
            block: Some(Block {
                header: Some(Header {
                    app_hash: vec![1, 2, 3, 4],
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let hash = extract_expected_app_hash(&request);
        assert_eq!(hash, Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn extract_expected_app_hash_no_block() {
        let request = RequestFinalizeBlock {
            block: None,
            ..Default::default()
        };
        assert_eq!(extract_expected_app_hash(&request), None);
    }

    #[test]
    fn extract_expected_app_hash_no_header() {
        let request = RequestFinalizeBlock {
            block: Some(Block {
                header: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(extract_expected_app_hash(&request), None);
    }

    // ---- LoadedRequest::block_height ----

    #[test]
    fn loaded_request_block_height_init_chain() {
        let request = LoadedRequest::InitChain(RequestInitChain::default());
        assert_eq!(request.block_height(), Some(0));
    }

    #[test]
    fn loaded_request_block_height_info() {
        let request = LoadedRequest::Info(RequestInfo::default());
        assert_eq!(request.block_height(), None);
    }

    #[test]
    fn loaded_request_block_height_prepare() {
        let request = LoadedRequest::Prepare(RequestPrepareProposal {
            height: 42,
            ..Default::default()
        });
        assert_eq!(request.block_height(), Some(42));
    }

    #[test]
    fn loaded_request_block_height_process() {
        let request = LoadedRequest::Process(RequestProcessProposal {
            height: 100,
            ..Default::default()
        });
        assert_eq!(request.block_height(), Some(100));
    }

    #[test]
    fn loaded_request_block_height_finalize() {
        let request = LoadedRequest::Finalize(RequestFinalizeBlock {
            height: 200,
            ..Default::default()
        });
        assert_eq!(request.block_height(), Some(200));
    }

    #[test]
    fn loaded_request_block_height_extend_vote() {
        let request = LoadedRequest::ExtendVote(RequestExtendVote {
            height: 300,
            ..Default::default()
        });
        assert_eq!(request.block_height(), Some(300));
    }

    #[test]
    fn loaded_request_block_height_verify_vote_extension() {
        let request = LoadedRequest::VerifyVoteExtension(RequestVerifyVoteExtension {
            height: 400,
            ..Default::default()
        });
        assert_eq!(request.block_height(), Some(400));
    }

    #[test]
    fn loaded_request_block_height_negative() {
        let request = LoadedRequest::Prepare(RequestPrepareProposal {
            height: -1,
            ..Default::default()
        });
        assert_eq!(request.block_height(), None);
    }

    // ---- ReplaySource::describe ----

    #[test]
    fn replay_source_describe_minimal() {
        let source = ReplaySource {
            path: PathBuf::from("/some/file.log"),
            line: 42,
            timestamp: None,
            endpoint: None,
        };
        assert_eq!(source.describe(), "/some/file.log:42");
    }

    #[test]
    fn replay_source_describe_with_endpoint() {
        let source = ReplaySource {
            path: PathBuf::from("/some/file.log"),
            line: 42,
            timestamp: None,
            endpoint: Some("process_proposal".to_string()),
        };
        assert_eq!(source.describe(), "/some/file.log:42 process_proposal");
    }

    #[test]
    fn replay_source_describe_with_timestamp() {
        let source = ReplaySource {
            path: PathBuf::from("/some/file.log"),
            line: 42,
            timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            endpoint: None,
        };
        assert_eq!(source.describe(), "/some/file.log:42 @2024-01-01T00:00:00Z");
    }

    #[test]
    fn replay_source_describe_with_both() {
        let source = ReplaySource {
            path: PathBuf::from("/some/file.log"),
            line: 42,
            timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            endpoint: Some("finalize_block".to_string()),
        };
        assert_eq!(
            source.describe(),
            "/some/file.log:42 finalize_block @2024-01-01T00:00:00Z"
        );
    }

    // ---- ReplayItem::from_log and describe ----

    #[test]
    fn replay_item_from_log_and_describe() {
        let item = ReplayItem::from_log(
            Path::new("/test/path.log"),
            10,
            Some("2024-01-01T00:00:00Z".to_string()),
            Some("info".to_string()),
            LoadedRequest::Info(RequestInfo::default()),
        );
        assert_eq!(
            item.describe(),
            "/test/path.log:10 info @2024-01-01T00:00:00Z"
        );
        assert_eq!(item.source.line(), 10);
    }

    // ---- ProgressReporter ----

    #[test]
    fn progress_reporter_format_rate_some() {
        let formatted = ProgressReporter::format_rate(Some(12.345));
        assert_eq!(formatted, "12.35 blk/min");
    }

    #[test]
    fn progress_reporter_format_rate_none() {
        let formatted = ProgressReporter::format_rate(None);
        assert_eq!(formatted, "n/a");
    }

    #[test]
    fn progress_reporter_format_eta_none() {
        let formatted = ProgressReporter::format_eta(None);
        assert_eq!(formatted, "n/a");
    }

    #[test]
    fn progress_reporter_format_eta_zero() {
        let formatted = ProgressReporter::format_eta(Some(Duration::from_secs(0)));
        assert_eq!(formatted, "00s");
    }

    #[test]
    fn progress_reporter_format_eta_seconds_only() {
        let formatted = ProgressReporter::format_eta(Some(Duration::from_secs(45)));
        assert_eq!(formatted, "45s");
    }

    #[test]
    fn progress_reporter_format_eta_minutes_and_seconds() {
        let formatted = ProgressReporter::format_eta(Some(Duration::from_secs(125)));
        assert_eq!(formatted, "02m05s");
    }

    #[test]
    fn progress_reporter_format_eta_hours() {
        let formatted = ProgressReporter::format_eta(Some(Duration::from_secs(3723)));
        assert_eq!(formatted, "01h02m03s");
    }

    #[test]
    fn progress_reporter_short_hash_truncates() {
        let hash = vec![0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xDE, 0xAD];
        let short = ProgressReporter::short_hash(&hash);
        assert_eq!(short.len(), 10);
        assert_eq!(short, "abcdef0123");
    }

    #[test]
    fn progress_reporter_short_hash_short_input() {
        let hash = vec![0xAB, 0xCD];
        let short = ProgressReporter::short_hash(&hash);
        assert_eq!(short, "abcd");
    }

    #[test]
    fn progress_reporter_eta_no_stop_height() {
        let reporter = ProgressReporter::new(None);
        assert_eq!(reporter.eta(Some(10.0), 50), None);
    }

    #[test]
    fn progress_reporter_eta_already_reached() {
        let reporter = ProgressReporter::new(Some(100));
        assert_eq!(reporter.eta(Some(10.0), 100), Some(Duration::from_secs(0)));
    }

    #[test]
    fn progress_reporter_eta_above_target() {
        let reporter = ProgressReporter::new(Some(100));
        assert_eq!(reporter.eta(Some(10.0), 200), Some(Duration::from_secs(0)));
    }

    #[test]
    fn progress_reporter_eta_no_rate() {
        let reporter = ProgressReporter::new(Some(100));
        assert_eq!(reporter.eta(None, 50), None);
    }

    #[test]
    fn progress_reporter_eta_zero_rate() {
        let reporter = ProgressReporter::new(Some(100));
        assert_eq!(reporter.eta(Some(0.0), 50), None);
    }

    #[test]
    fn progress_reporter_eta_negative_rate() {
        let reporter = ProgressReporter::new(Some(100));
        assert_eq!(reporter.eta(Some(-5.0), 50), None);
    }

    #[test]
    fn progress_reporter_eta_computes_remaining() {
        let reporter = ProgressReporter::new(Some(100));
        // 50 blocks remaining at 60 blocks/min = 50s
        let eta = reporter.eta(Some(60.0), 50);
        assert_eq!(eta, Some(Duration::from_secs(50)));
    }

    // ---- ensure_db_directory ----

    #[test]
    fn ensure_db_directory_creates_new() {
        let dir = tempfile::tempdir().unwrap();
        let new_path = dir.path().join("subdir").join("nested");
        let created = ensure_db_directory(&new_path).unwrap();
        assert!(created);
        assert!(new_path.is_dir());
    }

    #[test]
    fn ensure_db_directory_existing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let created = ensure_db_directory(dir.path()).unwrap();
        assert!(!created);
    }

    #[test]
    fn ensure_db_directory_existing_file_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("not_a_dir");
        fs::write(&file_path, "data").unwrap();
        let result = ensure_db_directory(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    // ---- ProgressReporter rate_per_minute and trim_history ----

    #[test]
    fn progress_reporter_rate_per_minute_empty_history() {
        let reporter = ProgressReporter::new(None);
        let rate = reporter.rate_per_minute(Instant::now(), Duration::from_secs(60));
        assert!(rate.is_none());
    }
}
