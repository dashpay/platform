use crate::abci::app::FullAbciApplication;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
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
use tenderdash_abci::Application;

#[derive(Debug, Clone)]
pub(super) struct ReplayItem {
    pub(super) source: ReplaySource,
    pub(super) request: LoadedRequest,
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
        }
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

pub(super) fn execute_request<C>(
    app: &FullAbciApplication<C>,
    item: ReplayItem,
    progress: Option<&mut ProgressReporter>,
) -> Result<Option<u64>, Box<dyn Error>>
where
    C: crate::rpc::core::CoreRPCLike,
{
    let origin = item.describe();
    let mut committed_height = None;
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
            let context = format_height_round(Some(height), Some(request.round));
            tracing::debug!(
                "executing process_proposal from {} (height={})",
                origin,
                height
            );
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
                hex::encode(response.app_hash),
                response.tx_results.len(),
                response.events.len()
            );
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

            let response = app.finalize_block(request).map_err(|err| {
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
            let grove_hash = app
                .platform
                .drive
                .grove
                .root_hash(
                    None,
                    &app.platform
                        .state
                        .load()
                        .current_platform_version()?
                        .drive
                        .grove_version,
                )
                .unwrap()
                .unwrap_or_default();
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
                    if expected_hash != &grove_hash {
                        return Err(logged_error(format!(
                            "grovedb root mismatch for {}{}: expected 0x{}, grove 0x{}",
                            origin,
                            context,
                            hex::encode(expected_hash),
                            hex::encode(grove_hash)
                        )));
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
    use tenderdash_abci::proto::abci::{
        RequestExtendVote, RequestFinalizeBlock, RequestInfo, RequestInitChain,
        RequestPrepareProposal, RequestProcessProposal, RequestVerifyVoteExtension,
    };
    use tenderdash_abci::proto::types::{Block, Header};

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
