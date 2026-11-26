mod chain_answer;
mod cli;
mod core_answers;
mod log_ingest;
mod report;
mod runner;

use crate::abci::app::FullAbciApplication;
use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::DefaultCoreRPC;
use crate::verify;
use cli::SkipSelector;
use core_answers::CoreAnswers;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use log_ingest::LogRequestStream;
use report::{DatabaseAt, ReplayChecks};
use runner::{
    ensure_db_directory, execute_request, log_last_committed_block, stop_height_reached,
    LoadedRequest, ProgressReporter, ReplayItem,
};
use std::error::Error;
use std::fs::{self, File};
use std::io;
use std::path::{self, Component, Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub use cli::ReplayArgs;

/// Replay ABCI requests captured in drive-abci JSON logs.
pub fn run(
    mut config: PlatformConfig,
    args: ReplayArgs,
    cancel: CancellationToken,
) -> Result<(), Box<dyn Error>> {
    config.db_path = database_directory(&config, &args)?;
    let db_is_new = database_is_new(&config.db_path)?;

    if db_is_new {
        // A new database has no platform state yet for a verification to read
        tracing::info!("the database is new; nothing to verify before replay");
    } else {
        tracing::info!("running database verification before replay");
        verify::run(&config, true)
            .map_err(|e| format!("verification failed before replay: {}", e))?;
    }

    let mut stream = LogRequestStream::open(&args.log)?;
    if stream.peek()?.is_some() {
        tracing::info!("streaming ABCI requests from log {}", args.log.display());
    } else {
        return Err(format!(
            "no supported ABCI requests found in log {}; provide --log with relevant inputs",
            args.log.display()
        )
        .into());
    }

    if db_is_new {
        let mut first_is_init_chain = false;
        advance_stream(&mut stream, None, &args.skip)?;
        pass_over_info_requests(&mut stream)?;
        if let Some(item) = stream.peek()? {
            first_is_init_chain = matches!(item.request, LoadedRequest::InitChain(_));
        }

        if !first_is_init_chain {
            return Err("the database is new; first replayed request must be init_chain".into());
        }
    }

    // What the replay only reads is looked at before anything is made, and what it writes is
    // kept out of the database directory, so that a replay that cannot start leaves nothing
    // behind that a second start would take for a database
    for written in [&args.report, &args.core_answers_record]
        .into_iter()
        .flatten()
    {
        if is_inside(written, &config.db_path)? {
            return Err(format!(
                "{} is inside the database directory {}; the replay's own files are kept outside it",
                written.display(),
                config.db_path.display()
            )
            .into());
        }
    }
    if let Some(other) = &args.compare_report {
        File::open(other)
            .map_err(|error| format!("cannot open report {}: {}", other.display(), error))?;
    }

    let (core_rpc, core_answers) = open_core_rpc(&config, &args)?;

    // For a new database the report files are opened before the database is made too
    let checks_of_new_database = if db_is_new {
        Some(open_checks(&args, None, core_answers.clone())?)
    } else {
        None
    };

    ensure_db_directory(&config.db_path)?;
    let platform: Platform<DefaultCoreRPC> =
        Platform::open_with_client(&config.db_path, Some(config.clone()), core_rpc, None)?;
    tracing::info!(
        storage_engine = %platform.drive.storage_engine(),
        "replaying on GroveDB's storage engine",
    );
    log_last_committed_block(&platform);
    let mut known_height = platform
        .state
        .load()
        .last_committed_block_info()
        .as_ref()
        .map(|info| info.basic_info().height);

    // Before a replay that has nothing left to do ends well: its report has to be the
    // database's too
    let mut checks = match checks_of_new_database {
        Some(checks) => checks,
        None => {
            let state = platform.state.load();
            let database = known_height.map(|height| DatabaseAt {
                height,
                app_hash: state
                    .last_committed_block_app_hash()
                    .map(|app_hash| app_hash.to_vec())
                    .unwrap_or_default(),
            });
            open_checks(&args, database.as_ref(), core_answers.clone())?
        }
    };

    if let Some(limit) = args.stop_height {
        if let Some(current) = known_height {
            if current >= limit {
                tracing::info!(
                    "current platform height {} is already at or above stop height {}; ending replay",
                    current,
                    limit
                );
                return Ok(());
            }
        }
    }

    let app = FullAbciApplication::new(&platform);
    let replayed = replay_requests(
        &app,
        &mut stream,
        &args,
        &cancel,
        &mut known_height,
        &mut checks,
    );

    // What the checks counted is said for a replay that failed too: the blocks it counts are
    // the ones that passed every check before it stopped.
    log_checks(&checks, core_answers.as_ref());
    replayed
}

/// The database directory of the replay, as the one location that everything after looks at,
/// keeps clear of the replay's files, verifies, makes and opens: a path with a part that does
/// not exist yet may lead to a directory that does (`new/../db`).
fn database_directory(
    config: &PlatformConfig,
    args: &ReplayArgs,
) -> Result<PathBuf, Box<dyn Error>> {
    let given = args.db_path.as_ref().unwrap_or(&config.db_path);
    resolved(given).map_err(|error| {
        format!(
            "cannot resolve the database directory {}: {}",
            given.display(),
            error
        )
        .into()
    })
}

/// Whether the database at `path` has yet to be made: its directory is missing, or is there
/// and empty.
fn database_is_new(path: &Path) -> Result<bool, Box<dyn Error>> {
    if !path.exists() {
        return Ok(true);
    }
    if !path.is_dir() {
        return Err(format!("{} exists but is not a directory", path.display()).into());
    }
    Ok(fs::read_dir(path)?.next().is_none())
}

/// Where `path` leads: every part of it that exists with its links followed, and the parts
/// that do not exist yet as they are given, a `..` after such a part taking it away again (as
/// it does when the directories are made).
fn resolved(path: &Path) -> io::Result<PathBuf> {
    let mut resolved = PathBuf::new();
    // How many parts at the end of `resolved` do not exist yet
    let mut missing = 0usize;
    for part in path::absolute(path)?.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir if missing > 0 => {
                resolved.pop();
                missing -= 1;
            }
            Component::Normal(_) if missing > 0 => {
                resolved.push(part);
                missing += 1;
            }
            Component::ParentDir | Component::Normal(_) => {
                let next = resolved.join(part);
                match fs::canonicalize(&next) {
                    Ok(found) => resolved = found,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        // A link to a file that is not there would be followed once the file
                        // is made
                        if next
                            .symlink_metadata()
                            .is_ok_and(|metadata| metadata.is_symlink())
                        {
                            return Err(error);
                        }
                        resolved = next;
                        missing = 1;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }
    Ok(resolved)
}

/// Whether `path` is or would be inside the directory `directory`, links followed.
fn is_inside(path: &Path, directory: &Path) -> Result<bool, Box<dyn Error>> {
    let (path, directory) = (
        resolved(path).map_err(|error| format!("cannot resolve {}: {}", path.display(), error))?,
        resolved(directory)
            .map_err(|error| format!("cannot resolve {}: {}", directory.display(), error))?,
    );
    Ok(path.starts_with(directory))
}

fn open_checks(
    args: &ReplayArgs,
    database: Option<&DatabaseAt>,
    core_answers: Option<CoreAnswers>,
) -> Result<ReplayChecks, Box<dyn Error>> {
    Ok(ReplayChecks::open(
        args.report.as_deref(),
        args.compare_report.as_deref(),
        database,
    )?
    .with_core_answers(core_answers))
}

/// The client block execution asks Dash Core through: one that keeps Core's answers in a
/// file, one that answers from such a file, or the node's own.
fn open_core_rpc(
    config: &PlatformConfig,
    args: &ReplayArgs,
) -> Result<(DefaultCoreRPC, Option<CoreAnswers>), Box<dyn Error>> {
    if let Some(path) = &args.core_answers {
        tracing::info!("answering Core RPC requests from {}", path.display());
        let (answers, core_rpc) = CoreAnswers::serve(path)?;
        return Ok((core_rpc, Some(answers)));
    }

    let rpc = &config.core.consensus_rpc;
    if let Some(path) = &args.core_answers_record {
        tracing::info!("recording Core RPC answers in {}", path.display());
        let (answers, core_rpc) = CoreAnswers::record(
            path,
            rpc.url().as_str(),
            rpc.username.clone(),
            rpc.password.clone(),
        )?;
        return Ok((core_rpc, Some(answers)));
    }

    let core_rpc = DefaultCoreRPC::open(
        rpc.url().as_str(),
        rpc.username.clone(),
        rpc.password.clone(),
    )?;
    Ok((core_rpc, None))
}

/// Executes the stream's requests until it ends, the stop height is reached or the replay is
/// cancelled.
fn replay_requests(
    app: &FullAbciApplication<DefaultCoreRPC>,
    stream: &mut LogRequestStream,
    args: &ReplayArgs,
    cancel: &CancellationToken,
    known_height: &mut Option<u64>,
    checks: &mut ReplayChecks,
) -> Result<(), Box<dyn Error>> {
    let mut progress = if args.progress {
        Some(ProgressReporter::new(args.stop_height))
    } else {
        None
    };
    let mut cancelled = false;
    let mut validator = RequestSequenceValidator::new(stream.path().to_path_buf());
    let mut executed = 0usize;
    loop {
        if cancel.is_cancelled() {
            tracing::info!(
                "cancellation requested; stopping replay for log {}",
                stream.path().display()
            );
            cancelled = true;
            break;
        }
        if stop_height_reached(args.stop_height, *known_height) {
            tracing::info!(
                "stop height {} reached; stopping replay for log {}",
                args.stop_height.unwrap(),
                stream.path().display()
            );
            break;
        }
        advance_stream(stream, *known_height, &args.skip)?;
        let Some(item) = stream.next_item()? else {
            break;
        };
        validator.observe(&item)?;
        let origin = item.describe();
        let executed_request = execute_request(app, item, progress.as_mut(), checks);
        // Block execution may read a failed request to Core as Core being away for a moment
        // and go on, so a failure of the answers' file ends the replay whatever the request
        // made of it
        checks
            .core_answers_failed()
            .map_err(|failure| format!("{} (at {})", failure, origin))?;
        update_known_height(known_height, executed_request?);
        executed += 1;
    }
    validator.finish()?;
    tracing::info!(
        "replayed {} ABCI requests from log {}",
        executed,
        stream.path().display()
    );

    if cancelled {
        tracing::info!("replay interrupted by cancellation");
    }

    Ok(())
}

/// Says what the replay's checks counted.
fn log_checks(checks: &ReplayChecks, core_answers: Option<&CoreAnswers>) {
    let summary = checks.summary();
    tracing::info!(
        blocks = summary.blocks,
        first_height = summary.first_height,
        last_height = summary.last_height,
        state_transitions = summary.transitions,
        blocks_held_against_logged_answers = summary.blocks_held_against_log,
        blocks_without_their_proposal_answer = summary.blocks_without_proposal,
        blocks_held_against_other_report = summary.blocks_held_against_report,
        proposals = summary.proposals,
        proposals_held_against_logged_answers = summary.proposals_held_against_log,
        proposals_without_logged_answer = summary.proposals_without_logged_answer,
        proposals_with_unreadable_logged_answer = summary.proposals_with_unreadable_logged_answer,
        info_requests_of_another_abci_version_passed_over = summary.info_passed_over,
        "replay checks",
    );
    tracing::info!(
        count = summary.root_not_app_hash.len(),
        heights = ?summary.root_not_app_hash,
        "heights after which the grovedb root is not the app hash",
    );
    if let Some(core_answers) = core_answers {
        let summary = core_answers.summary();
        tracing::info!(
            requests_held = summary.held,
            answers_added = summary.added,
            asked = ?summary.asked,
            "core answers",
        );
    }
}

/// Passes over `info` requests at the front of the stream. A node's log starts with the one
/// Tenderdash sends before `init_chain`, and a new database is given `init_chain` first.
fn pass_over_info_requests(stream: &mut LogRequestStream) -> Result<(), Box<dyn Error>> {
    while let Some(item) = stream.peek()? {
        if !matches!(item.request, LoadedRequest::Info(_)) {
            break;
        }
        let description = item.describe();
        stream.next_item()?;
        tracing::info!(
            "passing over info request {} before init_chain on a new database",
            description
        );
    }
    Ok(())
}

fn advance_stream(
    stream: &mut LogRequestStream,
    known_height: Option<u64>,
    skip: &[SkipSelector],
) -> Result<(), Box<dyn Error>> {
    if let Some(height) = known_height {
        let skipped = stream.skip_processed_entries(height)?;
        if skipped > 0 {
            tracing::info!(
                "skipped {} ABCI requests already applied (height <= {}) in log {}",
                skipped,
                height,
                stream.path().display()
            );
        }
    }
    drain_skipped_entries(stream, skip)?;
    Ok(())
}

fn drain_skipped_entries(
    stream: &mut LogRequestStream,
    skip: &[SkipSelector],
) -> Result<(), Box<dyn Error>> {
    if skip.is_empty() {
        return Ok(());
    }

    loop {
        let Some(item) = stream.peek()? else {
            break;
        };

        if should_skip_item(item, skip) {
            let description = item.describe();
            if stream.next_item()?.is_none() {
                break;
            }
            tracing::info!("skipping request {} due to --skip", description);
            continue;
        }

        break;
    }

    Ok(())
}

fn should_skip_item(item: &ReplayItem, skip: &[SkipSelector]) -> bool {
    let line = item.source.line();
    skip.iter().any(|selector| selector.matches(line))
}

fn update_known_height(current: &mut Option<u64>, new_height: Option<u64>) {
    if let Some(height) = new_height {
        match current {
            Some(existing) if height <= *existing => {}
            _ => *current = Some(height),
        }
    }
}

struct RequestSequenceValidator {
    path: PathBuf,
    last_height: Option<u64>,
    saw_process: bool,
    saw_finalize: bool,
}

impl RequestSequenceValidator {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            last_height: None,
            saw_process: false,
            saw_finalize: false,
        }
    }

    fn observe(&mut self, item: &ReplayItem) -> Result<(), Box<dyn Error>> {
        let height = match item.request.block_height() {
            Some(height) => height,
            None => return Ok(()),
        };

        match &item.request {
            LoadedRequest::Process(_) => self.record_process(height, &item.describe()),
            LoadedRequest::Finalize(_) => self.record_finalize(height, &item.describe()),
            _ => Ok(()),
        }
    }

    fn record_process(&mut self, height: u64, origin: &str) -> Result<(), Box<dyn Error>> {
        self.bump_height(height, origin)?;
        self.saw_process = true;
        Ok(())
    }

    fn record_finalize(&mut self, height: u64, origin: &str) -> Result<(), Box<dyn Error>> {
        self.bump_height(height, origin)?;
        if !self.saw_process {
            return Err(format!(
                "log {} contains finalize_block before process_proposal at height {} ({})",
                self.path.display(),
                height,
                origin
            )
            .into());
        }
        self.saw_finalize = true;
        Ok(())
    }

    fn bump_height(&mut self, height: u64, origin: &str) -> Result<(), Box<dyn Error>> {
        match self.last_height {
            Some(last) if height < last => Err(format!(
                "log {} has out-of-order height: encountered {} after {} ({})",
                self.path.display(),
                height,
                last,
                origin
            )
            .into()),
            Some(last) if height == last => Ok(()),
            Some(last) => {
                if !self.saw_process || !self.saw_finalize {
                    return Err(format!(
                        "log {} missing process/finalize pair for height {} before {}",
                        self.path.display(),
                        last,
                        origin
                    )
                    .into());
                }
                if height != last + 1 {
                    return Err(format!(
                        "log {} skipped heights ({} -> {}) before {}",
                        self.path.display(),
                        last,
                        height,
                        origin
                    )
                    .into());
                }
                self.last_height = Some(height);
                self.saw_process = false;
                self.saw_finalize = false;
                Ok(())
            }
            None => {
                self.last_height = Some(height);
                Ok(())
            }
        }
    }

    fn finish(&self) -> Result<(), Box<dyn Error>> {
        if let Some(last_height) = self.last_height {
            if !self.saw_process || !self.saw_finalize {
                return Err(format!(
                    "log {} ended before height {} had both process_proposal and finalize_block",
                    self.path.display(),
                    last_height
                )
                .into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_answer::{ChainProposalAnswer, LoggedAnswer, TxOutcome};
    use runner::LoadedRequest;
    use std::io::Write;
    use std::iter;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use tenderdash_abci::proto::abci::{
        RequestFinalizeBlock, RequestInfo, RequestInitChain, RequestProcessProposal,
    };

    // ---- update_known_height ----

    #[test]
    fn update_known_height_from_none() {
        let mut current = None;
        update_known_height(&mut current, Some(10));
        assert_eq!(current, Some(10));
    }

    #[test]
    fn update_known_height_higher() {
        let mut current = Some(5);
        update_known_height(&mut current, Some(10));
        assert_eq!(current, Some(10));
    }

    #[test]
    fn update_known_height_lower_ignored() {
        let mut current = Some(10);
        update_known_height(&mut current, Some(5));
        assert_eq!(current, Some(10));
    }

    #[test]
    fn update_known_height_equal_ignored() {
        let mut current = Some(10);
        update_known_height(&mut current, Some(10));
        assert_eq!(current, Some(10));
    }

    #[test]
    fn update_known_height_none_new_ignored() {
        let mut current = Some(10);
        update_known_height(&mut current, None);
        assert_eq!(current, Some(10));
    }

    #[test]
    fn update_known_height_both_none() {
        let mut current = None;
        update_known_height(&mut current, None);
        assert_eq!(current, None);
    }

    // ---- should_skip_item ----

    fn make_replay_item(line: usize, request: LoadedRequest) -> ReplayItem {
        ReplayItem::from_log(std::path::Path::new("/test.log"), line, None, None, request)
    }

    #[test]
    fn should_skip_item_no_selectors() {
        let item = make_replay_item(10, LoadedRequest::Info(RequestInfo::default()));
        assert!(!should_skip_item(&item, &[]));
    }

    #[test]
    fn should_skip_item_matching_line() {
        let item = make_replay_item(10, LoadedRequest::Info(RequestInfo::default()));
        let skip = vec![SkipSelector::Line(10)];
        assert!(should_skip_item(&item, &skip));
    }

    #[test]
    fn should_skip_item_non_matching_line() {
        let item = make_replay_item(10, LoadedRequest::Info(RequestInfo::default()));
        let skip = vec![SkipSelector::Line(11)];
        assert!(!should_skip_item(&item, &skip));
    }

    #[test]
    fn should_skip_item_in_range() {
        let item = make_replay_item(15, LoadedRequest::Info(RequestInfo::default()));
        let skip = vec![SkipSelector::Range { start: 10, end: 20 }];
        assert!(should_skip_item(&item, &skip));
    }

    #[test]
    fn should_skip_item_outside_range() {
        let item = make_replay_item(25, LoadedRequest::Info(RequestInfo::default()));
        let skip = vec![SkipSelector::Range { start: 10, end: 20 }];
        assert!(!should_skip_item(&item, &skip));
    }

    #[test]
    fn should_skip_item_multiple_selectors() {
        let item = make_replay_item(25, LoadedRequest::Info(RequestInfo::default()));
        let skip = vec![
            SkipSelector::Line(10),
            SkipSelector::Range { start: 20, end: 30 },
        ];
        assert!(should_skip_item(&item, &skip));
    }

    // ---- RequestSequenceValidator ----

    #[test]
    fn validator_empty_sequence_ok() {
        let validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));
        validator.finish().unwrap();
    }

    #[test]
    fn validator_complete_height_ok() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));

        let process = make_replay_item(
            1,
            LoadedRequest::Process(RequestProcessProposal {
                height: 1,
                ..Default::default()
            }),
        );
        let finalize = make_replay_item(
            2,
            LoadedRequest::Finalize(RequestFinalizeBlock {
                height: 1,
                ..Default::default()
            }),
        );

        validator.observe(&process).unwrap();
        validator.observe(&finalize).unwrap();
        validator.finish().unwrap();
    }

    #[test]
    fn validator_two_consecutive_heights_ok() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));

        let items = [
            make_replay_item(
                1,
                LoadedRequest::Process(RequestProcessProposal {
                    height: 1,
                    ..Default::default()
                }),
            ),
            make_replay_item(
                2,
                LoadedRequest::Finalize(RequestFinalizeBlock {
                    height: 1,
                    ..Default::default()
                }),
            ),
            make_replay_item(
                3,
                LoadedRequest::Process(RequestProcessProposal {
                    height: 2,
                    ..Default::default()
                }),
            ),
            make_replay_item(
                4,
                LoadedRequest::Finalize(RequestFinalizeBlock {
                    height: 2,
                    ..Default::default()
                }),
            ),
        ];

        for item in &items {
            validator.observe(item).unwrap();
        }
        validator.finish().unwrap();
    }

    #[test]
    fn validator_finalize_before_process_error() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));
        let finalize = make_replay_item(
            1,
            LoadedRequest::Finalize(RequestFinalizeBlock {
                height: 1,
                ..Default::default()
            }),
        );
        let err = validator.observe(&finalize).unwrap_err();
        assert!(
            err.to_string()
                .contains("finalize_block before process_proposal"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validator_out_of_order_height_error() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));

        let items = [
            make_replay_item(
                1,
                LoadedRequest::Process(RequestProcessProposal {
                    height: 5,
                    ..Default::default()
                }),
            ),
            make_replay_item(
                2,
                LoadedRequest::Finalize(RequestFinalizeBlock {
                    height: 5,
                    ..Default::default()
                }),
            ),
        ];
        for item in &items {
            validator.observe(item).unwrap();
        }

        let bad = make_replay_item(
            3,
            LoadedRequest::Process(RequestProcessProposal {
                height: 3,
                ..Default::default()
            }),
        );
        let err = validator.observe(&bad).unwrap_err();
        assert!(
            err.to_string().contains("out-of-order height"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validator_skipped_heights_error() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));

        let items = [
            make_replay_item(
                1,
                LoadedRequest::Process(RequestProcessProposal {
                    height: 1,
                    ..Default::default()
                }),
            ),
            make_replay_item(
                2,
                LoadedRequest::Finalize(RequestFinalizeBlock {
                    height: 1,
                    ..Default::default()
                }),
            ),
        ];
        for item in &items {
            validator.observe(item).unwrap();
        }

        let skip = make_replay_item(
            3,
            LoadedRequest::Process(RequestProcessProposal {
                height: 3,
                ..Default::default()
            }),
        );
        let err = validator.observe(&skip).unwrap_err();
        assert!(
            err.to_string().contains("skipped heights"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validator_missing_pair_on_height_change_error() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));

        // Only process, no finalize for height 1
        let process = make_replay_item(
            1,
            LoadedRequest::Process(RequestProcessProposal {
                height: 1,
                ..Default::default()
            }),
        );
        validator.observe(&process).unwrap();

        // Jump to height 2
        let process2 = make_replay_item(
            2,
            LoadedRequest::Process(RequestProcessProposal {
                height: 2,
                ..Default::default()
            }),
        );
        let err = validator.observe(&process2).unwrap_err();
        assert!(
            err.to_string().contains("missing process/finalize pair"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validator_finish_incomplete_height_error() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));

        let process = make_replay_item(
            1,
            LoadedRequest::Process(RequestProcessProposal {
                height: 1,
                ..Default::default()
            }),
        );
        validator.observe(&process).unwrap();

        let err = validator.finish().unwrap_err();
        assert!(
            err.to_string()
                .contains("ended before height 1 had both process_proposal and finalize_block"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn validator_info_request_ignored() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));
        let info = make_replay_item(1, LoadedRequest::Info(RequestInfo::default()));
        validator.observe(&info).unwrap();
        validator.finish().unwrap();
    }

    #[test]
    fn validator_init_chain_ignored() {
        let mut validator = RequestSequenceValidator::new(PathBuf::from("/test.log"));
        // InitChain returns height 0, which is treated as no block_height match (not Process/Finalize)
        let init = make_replay_item(1, LoadedRequest::InitChain(RequestInitChain::default()));
        validator.observe(&init).unwrap();
        // InitChain is neither Process nor Finalize, so no error
    }

    // ---- LogRequestStream integration tests ----

    #[test]
    fn log_stream_reads_info_request() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            writeln!(
                f,
                r#"{{"timestamp":"2024-01-01T00:00:00Z","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(Info(RequestInfo {{ version: \"1.0\", block_version: 14, p2p_version: 10, abci_version: \"1.0\" }})) }}"}},"span":{{"endpoint":"info"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        let item = stream.next_item().unwrap().expect("expected an item");
        assert!(matches!(item.request, LoadedRequest::Info(_)));
        assert!(stream.next_item().unwrap().is_none());
    }

    #[test]
    fn log_stream_skips_irrelevant_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            // Line without "received ABCI request"
            writeln!(
                f,
                r#"{{"timestamp":"2024-01-01T00:00:00Z","fields":{{"message":"something else"}}}}"#
            )
            .unwrap();
            // Empty line
            writeln!(f).unwrap();
            // Malformed JSON
            writeln!(f, "not json at all").unwrap();
            // Line without fields
            writeln!(f, r#"{{"timestamp":"2024-01-01T00:00:00Z"}}"#).unwrap();
            // Line with message but no request
            writeln!(
                f,
                r#"{{"timestamp":"2024-01-01T00:00:00Z","fields":{{"message":"received ABCI request"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        assert!(stream.next_item().unwrap().is_none());
    }

    #[test]
    fn log_stream_peek_then_next() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            writeln!(
                f,
                r#"{{"timestamp":"2024-01-01T00:00:00Z","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(Info(RequestInfo {{ version: \"1.0\", block_version: 14, p2p_version: 10, abci_version: \"1.0\" }})) }}"}},"span":{{"endpoint":"info"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        // Peek should return a reference without consuming
        assert!(stream.peek().unwrap().is_some());
        // Peek again should return the same item
        assert!(stream.peek().unwrap().is_some());
        // next_item should consume the buffered item
        let item = stream.next_item().unwrap().expect("expected an item");
        assert!(matches!(item.request, LoadedRequest::Info(_)));
        // Stream should now be empty
        assert!(stream.next_item().unwrap().is_none());
    }

    #[test]
    fn log_stream_skip_processed_entries() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            // ProcessProposal at height 1
            writeln!(
                f,
                r#"{{"timestamp":"t1","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(ProcessProposal(RequestProcessProposal {{ txs: [], proposed_last_commit: None, misbehavior: [], hash: [], height: 1, round: 0, time: None, next_validators_hash: [], core_chain_locked_height: 0, core_chain_lock_update: None, proposer_pro_tx_hash: [], proposed_app_version: 1, version: None, quorum_hash: [] }})) }}"}}}}"#
            )
            .unwrap();
            // ProcessProposal at height 5
            writeln!(
                f,
                r#"{{"timestamp":"t2","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(ProcessProposal(RequestProcessProposal {{ txs: [], proposed_last_commit: None, misbehavior: [], hash: [], height: 5, round: 0, time: None, next_validators_hash: [], core_chain_locked_height: 0, core_chain_lock_update: None, proposer_pro_tx_hash: [], proposed_app_version: 1, version: None, quorum_hash: [] }})) }}"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        let skipped = stream.skip_processed_entries(3).unwrap();
        assert_eq!(skipped, 1);
        // Remaining item should be height 5
        let item = stream.next_item().unwrap().expect("expected height 5 item");
        assert_eq!(item.request.block_height(), Some(5));
    }

    // ---- the logged answers, a new database, the Core client ----

    const INFO: &str = r#"Request { value: Some(Info(RequestInfo { version: "1.0", block_version: 14, p2p_version: 10, abci_version: "1.0" })) }"#;

    const INIT_CHAIN: &str = r#"Request { value: Some(InitChain(RequestInitChain { time: None, chain_id: "test", consensus_params: None, validator_set: None, app_state_bytes: [], initial_height: 1, initial_core_height: 0 })) }"#;

    /// The answer of a node to a proposal with one state transition, as its log has it.
    const ACCEPTED: &str = r#""\"status\" \" \"Accepted\"\", \"app_hash\" \" \"26a5\"\", \"tx_results\" \" [\"\\\"code\\\" \\\" 0\\\", \\\"data\\\" \\\" \\\"\\\"\\\", \\\"log\\\" \\\" \\\"\\\"\\\", \\\"info\\\" \\\" \\\"\\\"\\\", \\\"gas_used\\\"\\n\\\" 117438280\\\", \\\"events\\\" \\\" \\\"[]\\\"\\\", \\\"codespace\\\" \\\" \\\"\\\"\\\",\"]\",\n\"consensus_param_updates\" \" \"None\"\", \"validator_set_update\" \" \"None\"\",""#;

    fn process_proposal(height: u64) -> String {
        format!(
            "Request {{ value: Some(ProcessProposal(RequestProcessProposal {{ txs: [], proposed_last_commit: None, misbehavior: [], hash: [], height: {height}, round: 0, time: None, next_validators_hash: [], core_chain_locked_height: 0, core_chain_lock_update: None, proposer_pro_tx_hash: [], proposed_app_version: 1, version: None, quorum_hash: [] }})) }}"
        )
    }

    /// A request line that the thread `thread` logged.
    fn request_line_of(thread: u64, request: &str) -> String {
        serde_json::json!({
            "timestamp": "t",
            "fields": {"message": "received ABCI request", "request": request},
            "threadName": "main",
            "threadId": format!("ThreadId({thread})"),
        })
        .to_string()
    }

    /// An answer line that the thread `thread` logged.
    fn answer_line_of(thread: u64, response_log: &str) -> String {
        serde_json::json!({
            "timestamp": "t",
            "fields": {"message": "sending ABCI response", "response_log": response_log},
            "threadName": "main",
            "threadId": format!("ThreadId({thread})"),
        })
        .to_string()
    }

    fn request_line(request: &str) -> String {
        request_line_of(1, request)
    }

    fn answer_line(response_log: &str) -> String {
        answer_line_of(1, response_log)
    }

    fn log_of(lines: &[String]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        fs::write(&log_path, lines.join("\n")).unwrap();
        (dir, log_path)
    }

    const INFO_ANSWER: &str = "\"Info(ResponseInfo { data: \\\"\\\" })\"";

    fn of_another_thread(line: usize, what: &str) -> LoggedAnswer {
        LoggedAnswer::Unreadable(format!(
            "line {line} of the log, after the request, is {what} that does not name the request's thread"
        ))
    }

    fn proposal_answers(lines: &[String]) -> Vec<LoggedAnswer> {
        let (_dir, log_path) = log_of(lines);
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        iter::from_fn(|| stream.next_item().unwrap())
            .filter(|item| matches!(item.request, LoadedRequest::Process(_)))
            .map(|item| item.logged_answer)
            .collect()
    }

    fn accepted() -> LoggedAnswer {
        LoggedAnswer::Proposal(ChainProposalAnswer {
            status: "Accepted".to_owned(),
            app_hash: "26a5".to_owned(),
            tx: vec![TxOutcome(0, 117438280)],
        })
    }

    #[test]
    fn should_give_a_process_proposal_the_answer_the_log_has_for_it() {
        let (_dir, log_path) = log_of(&[
            request_line(INFO),
            answer_line("\"Info(ResponseInfo { data: \\\"\\\" })\""),
            request_line(&process_proposal(1)),
            // Lines between a request and its answer that are neither
            r#"{"timestamp":"t","fields":{"message":"something else"}}"#.to_owned(),
            String::new(),
            r#"{"timestamp":"t"}"#.to_owned(),
            answer_line(ACCEPTED),
            request_line(&process_proposal(2)),
            answer_line(ACCEPTED),
        ]);
        let mut stream = LogRequestStream::open(&log_path).unwrap();

        // Only a proposal is given an answer
        let info = stream.next_item().unwrap().unwrap();
        assert!(matches!(info.request, LoadedRequest::Info(_)));
        assert_eq!(info.logged_answer, LoggedAnswer::Absent);

        let first = stream.next_item().unwrap().unwrap();
        assert_eq!(first.request.block_height(), Some(1));
        assert_eq!(first.source.line(), 3);
        assert_eq!(first.logged_answer, accepted());

        let second = stream.next_item().unwrap().unwrap();
        assert_eq!(second.request.block_height(), Some(2));
        assert_eq!(second.source.line(), 8);
        assert_eq!(second.logged_answer, accepted());

        assert!(stream.next_item().unwrap().is_none());
    }

    #[test]
    fn should_not_take_an_answer_after_a_line_that_cannot_be_read() {
        // The line that cannot be read may have been the answer, or the next request
        let (_dir, log_path) = log_of(&[
            request_line(&process_proposal(1)),
            "{\"timestamp\":\"t\",\"fields\":{\"message\":\"sending ABCI resp".to_owned(),
            answer_line(ACCEPTED),
            request_line(&process_proposal(2)),
            "not json at all".to_owned(),
            request_line(&process_proposal(3)),
        ]);
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        let first = stream.next_item().unwrap().unwrap();
        assert_eq!(
            first.logged_answer,
            LoggedAnswer::Unreadable(
                "line 2 of the log, after the request, cannot be read".to_owned()
            )
        );
        let second = stream.next_item().unwrap().unwrap();
        assert_eq!(second.request.block_height(), Some(2));
        assert_eq!(
            second.logged_answer,
            LoggedAnswer::Unreadable(
                "line 5 of the log, after the request, cannot be read".to_owned()
            )
        );
        let third = stream.next_item().unwrap().unwrap();
        assert_eq!(third.request.block_height(), Some(3));
        assert_eq!(third.source.line(), 6);
    }

    #[test]
    fn should_take_for_a_request_only_the_answer_its_thread_logged_next() {
        // Two connections, each one request after the other, whose lines lie between each
        // other's: thread 1 has P1, A1, Info, A(Info), thread 2 has Info, A(Info), P2, A2
        assert_eq!(
            proposal_answers(&[
                request_line_of(1, &process_proposal(1)),
                request_line_of(2, INFO),
                answer_line_of(2, INFO_ANSWER),
                request_line_of(2, &process_proposal(1)),
                answer_line_of(1, ACCEPTED),
                request_line_of(1, INFO),
                answer_line_of(2, ACCEPTED),
                answer_line_of(1, INFO_ANSWER),
            ]),
            vec![
                of_another_thread(2, "a request"),
                of_another_thread(5, "an answer"),
            ]
        );
        // Two proposals of two threads, the first answered after the second was asked
        assert_eq!(
            proposal_answers(&[
                request_line_of(1, &process_proposal(1)),
                request_line_of(2, &process_proposal(1)),
                answer_line_of(1, ACCEPTED),
                answer_line_of(2, ACCEPTED),
            ]),
            vec![
                of_another_thread(2, "a request"),
                of_another_thread(3, "an answer"),
            ]
        );
        // One after the other, on two threads
        assert_eq!(
            proposal_answers(&[
                request_line_of(1, &process_proposal(1)),
                answer_line_of(1, ACCEPTED),
                request_line_of(2, &process_proposal(2)),
                answer_line_of(2, ACCEPTED),
            ]),
            vec![accepted(), accepted()]
        );
        // The same thread id under another thread name is another thread
        let other_name = answer_line(ACCEPTED).replace("\"main\"", "\"worker\"");
        assert_eq!(
            proposal_answers(&[request_line(&process_proposal(1)), other_name]),
            vec![of_another_thread(2, "an answer")]
        );
    }

    #[test]
    fn should_take_no_answer_where_the_log_names_no_thread() {
        let unnamed = |line: String| {
            let mut value: serde_json::Value = serde_json::from_str(&line).unwrap();
            value.as_object_mut().unwrap().remove("threadId");
            value.to_string()
        };
        assert_eq!(
            proposal_answers(&[
                // The request's line names none
                unnamed(request_line(&process_proposal(1))),
                answer_line(ACCEPTED),
                // The answer's line names none
                request_line(&process_proposal(2)),
                unnamed(answer_line(ACCEPTED)),
                // Neither does
                unnamed(request_line(&process_proposal(3))),
                unnamed(answer_line(ACCEPTED)),
            ]),
            vec![
                LoggedAnswer::Unreadable(
                    "line 1 of the log does not name the thread that logged it, so no answer can be told to be this request's"
                        .to_owned()
                ),
                of_another_thread(4, "an answer"),
                LoggedAnswer::Unreadable(
                    "line 5 of the log does not name the thread that logged it, so no answer can be told to be this request's"
                        .to_owned()
                ),
            ]
        );
    }

    #[test]
    fn should_take_no_answer_for_a_request_a_node_was_stopped_in() {
        let exception = serde_json::json!({
            "timestamp": "t",
            "fields": {"message": "sending ABCI exception", "response": "Exception"},
            "threadName": "main",
            "threadId": "ThreadId(1)",
        })
        .to_string();
        assert_eq!(
            proposal_answers(&[
                // The node was stopped in a request: its next start asks for info first, and
                // the proposal that follows is answered as any other
                request_line(&process_proposal(1)),
                request_line(INFO),
                answer_line(INFO_ANSWER),
                request_line(&process_proposal(1)),
                answer_line(ACCEPTED),
                // The node answered with an error
                request_line(&process_proposal(2)),
                exception,
                // The log ends in a request
                request_line(&process_proposal(3)),
            ]),
            vec![
                LoggedAnswer::Absent,
                accepted(),
                LoggedAnswer::Absent,
                LoggedAnswer::Absent,
            ]
        );
    }

    #[test]
    fn should_keep_a_request_that_comes_before_any_answer() {
        let (_dir, log_path) = log_of(&[
            request_line(&process_proposal(1)),
            request_line(&process_proposal(2)),
            answer_line(ACCEPTED),
            request_line(&process_proposal(3)),
        ]);
        let mut stream = LogRequestStream::open(&log_path).unwrap();

        // The line read while looking for the first request's answer is the second request
        let first = stream.next_item().unwrap().unwrap();
        assert_eq!(first.request.block_height(), Some(1));
        assert_eq!(first.source.line(), 1);
        assert_eq!(first.logged_answer, LoggedAnswer::Absent);

        let second = stream.next_item().unwrap().unwrap();
        assert_eq!(second.request.block_height(), Some(2));
        assert_eq!(second.source.line(), 2);
        assert_eq!(second.logged_answer, accepted());

        // The log ends after the third request
        let third = stream.next_item().unwrap().unwrap();
        assert_eq!(third.request.block_height(), Some(3));
        assert_eq!(third.source.line(), 4);
        assert_eq!(third.logged_answer, LoggedAnswer::Absent);

        assert!(stream.next_item().unwrap().is_none());
    }

    #[test]
    fn should_say_that_an_answer_is_of_another_kind_or_cannot_be_read() {
        let (_dir, log_path) = log_of(&[
            request_line(&process_proposal(1)),
            answer_line("\"FinalizeBlock(ResponseFinalizeBlock { retain_height: 0 })\""),
            request_line(&process_proposal(2)),
            // An answer the log cut short
            answer_line("\"\\\"status\\\" \\\" \\\"Accepted"),
            request_line(&process_proposal(3)),
            // An answer line with no answer in it
            r#"{"timestamp":"t","fields":{"message":"sending ABCI response"},"threadName":"main","threadId":"ThreadId(1)"}"#.to_owned(),
        ]);
        let mut stream = LogRequestStream::open(&log_path).unwrap();

        let first = stream.next_item().unwrap().unwrap();
        assert_eq!(first.logged_answer, LoggedAnswer::Absent);
        let second = stream.next_item().unwrap().unwrap();
        assert_eq!(
            second.logged_answer,
            LoggedAnswer::Unreadable("a string does not end".to_owned())
        );
        let third = stream.next_item().unwrap().unwrap();
        assert_eq!(third.logged_answer, LoggedAnswer::Absent);
        assert!(stream.next_item().unwrap().is_none());
    }

    #[test]
    fn should_pass_over_info_requests_before_init_chain() {
        let (_dir, log_path) = log_of(&[
            request_line(INFO),
            request_line(INFO),
            request_line(INIT_CHAIN),
            request_line(INFO),
        ]);
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        pass_over_info_requests(&mut stream).unwrap();
        let item = stream.next_item().unwrap().unwrap();
        assert!(matches!(item.request, LoadedRequest::InitChain(_)));
        assert_eq!(item.source.line(), 3);
        // An info request after init_chain stays
        let item = stream.next_item().unwrap().unwrap();
        assert!(matches!(item.request, LoadedRequest::Info(_)));

        // A log of info requests only is passed over whole
        let (_dir, log_path) = log_of(&[request_line(INFO)]);
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        pass_over_info_requests(&mut stream).unwrap();
        assert!(stream.next_item().unwrap().is_none());
    }

    fn replay_args(log: &Path) -> ReplayArgs {
        ReplayArgs {
            db_path: None,
            log: log.to_path_buf(),
            progress: false,
            skip: vec![],
            stop_height: None,
            core_answers_record: None,
            core_answers: None,
            report: None,
            compare_report: None,
        }
    }

    #[test]
    fn should_take_a_missing_or_an_empty_directory_for_a_new_database() {
        let dir = tempfile::tempdir().unwrap();

        let missing = dir.path().join("missing");
        assert!(database_is_new(&missing).unwrap());
        // Asking makes nothing
        assert!(!missing.exists());

        // A replay that could not start can leave the directory behind, empty
        assert!(ensure_db_directory(&missing).unwrap());
        assert!(database_is_new(&missing).unwrap());

        fs::write(missing.join("CURRENT"), "MANIFEST-000001").unwrap();
        assert!(!database_is_new(&missing).unwrap());

        let file = dir.path().join("file");
        fs::write(&file, "").unwrap();
        let error = database_is_new(&file).unwrap_err().to_string();
        assert!(error.contains("exists but is not a directory"), "{error}");
    }

    #[test]
    fn should_refuse_the_report_files_before_a_new_database_is_made() {
        let dir = tempfile::tempdir().unwrap();
        let report = dir.path().join("report.jsonl");
        let args = ReplayArgs {
            report: Some(report.clone()),
            compare_report: Some(report.clone()),
            ..replay_args(&dir.path().join("abci.jsonl"))
        };
        let error = open_checks(&args, None, None).unwrap_err().to_string();
        assert!(error.contains("cannot be held against itself"), "{error}");

        let args = ReplayArgs {
            compare_report: Some(dir.path().join("missing.jsonl")),
            ..args
        };
        let error = open_checks(&args, None, None).unwrap_err().to_string();
        assert!(error.starts_with("cannot open report "), "{error}");
        assert!(!report.exists());
    }

    #[test]
    fn should_look_at_the_database_directory_that_is_opened() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("db");
        fs::create_dir(&db).unwrap();
        fs::write(db.join("CURRENT"), "a database").unwrap();
        // A database that is there, named through a directory that is not: making the
        // directories leads to it, so it is not a new database
        let named = dir.path().join("new").join("..").join("db");
        let args = ReplayArgs {
            db_path: Some(named),
            ..replay_args(Path::new("abci.jsonl"))
        };
        let config = PlatformConfig::default_testnet();
        let opened = database_directory(&config, &args).unwrap();
        assert_eq!(opened, fs::canonicalize(&db).unwrap());
        assert!(!database_is_new(&opened).unwrap());

        // Without the option it is the configuration's
        let args = replay_args(Path::new("abci.jsonl"));
        let config = PlatformConfig {
            db_path: dir.path().join(".").join("db"),
            ..config
        };
        assert_eq!(
            database_directory(&config, &args).unwrap(),
            fs::canonicalize(&db).unwrap()
        );
    }

    #[test]
    fn should_tell_a_file_inside_the_database_directory() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("db");
        assert!(is_inside(&db.join("report.jsonl"), &db).unwrap());
        assert!(is_inside(&db.join("deeper").join("report.jsonl"), &db).unwrap());
        assert!(!is_inside(&dir.path().join("report.jsonl"), &db).unwrap());
        // A name that begins as the directory's does is beside it
        assert!(!is_inside(&dir.path().join("db-report.jsonl"), &db).unwrap());

        // Once the directory is there: through a link to it, and back out of it
        fs::create_dir(&db).unwrap();
        #[cfg(unix)]
        {
            let link = dir.path().join("out");
            symlink(&db, &link).unwrap();
            assert!(is_inside(&link.join("core-answers.jsonl"), &db).unwrap());
            // A link beside the directory to a file that would be made inside it
            let to_be_made = dir.path().join("report.jsonl");
            symlink(db.join("report.jsonl"), &to_be_made).unwrap();
            let error = is_inside(&to_be_made, &db).unwrap_err().to_string();
            assert!(error.starts_with("cannot resolve "), "{error}");
        }
        assert!(!is_inside(&db.join("..").join("beside.jsonl"), &db).unwrap());
        // A database directory that is made through a directory that is not there yet
        let to_be = dir.path().join("new").join("..").join("to-be");
        assert!(!is_inside(&dir.path().join("report.txt"), &to_be).unwrap());
        assert!(is_inside(&dir.path().join("to-be").join("report.txt"), &to_be).unwrap());
        assert!(is_inside(&to_be.join("report.txt"), &dir.path().join("to-be")).unwrap());
        assert!(!is_inside(&dir.path().join("new").join("report.txt"), &to_be).unwrap());
        assert!(is_inside(&dir.path().join(".").join("db").join("r"), &db).unwrap());
    }

    #[test]
    fn should_stop_before_anything_is_made_where_a_start_cannot_succeed() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("abci.jsonl");
        fs::write(
            &log,
            format!("{}\n{}\n", request_line(INFO), request_line(INIT_CHAIN)),
        )
        .unwrap();
        let db = dir.path().join("db");
        let answers = dir.path().join("core-answers.jsonl");
        let report = dir.path().join("report.jsonl");
        let mut config = PlatformConfig::default_testnet();
        config.core.consensus_rpc.host = "127.0.0.1".to_owned();
        let args = ReplayArgs {
            db_path: Some(db.clone()),
            core_answers_record: Some(answers.clone()),
            report: Some(report.clone()),
            compare_report: Some(dir.path().join("missing.jsonl")),
            ..replay_args(&log)
        };

        // The other report is not there
        let error = run(config.clone(), args.clone(), CancellationToken::new())
            .unwrap_err()
            .to_string();
        assert!(error.starts_with("cannot open report "), "{error}");
        assert!(!db.exists() && !answers.exists() && !report.exists());

        // A file the replay writes is inside the database directory
        for inside in [
            ReplayArgs {
                report: Some(db.join("report.jsonl")),
                compare_report: None,
                ..args.clone()
            },
            ReplayArgs {
                core_answers_record: Some(db.join("core-answers.jsonl")),
                compare_report: None,
                ..args.clone()
            },
        ] {
            let error = run(config.clone(), inside, CancellationToken::new())
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("the replay's own files are kept outside it"),
                "{error}"
            );
            assert!(!db.exists() && !answers.exists() && !report.exists());
        }
    }

    #[test]
    fn should_open_the_core_client_the_arguments_name() {
        let dir = tempfile::tempdir().unwrap();
        let answers_path = dir.path().join("core-answers.jsonl");
        let mut config = PlatformConfig::default_testnet();
        // Nothing is asked of it: a client connects when it is first asked
        config.core.consensus_rpc.host = "127.0.0.1".to_owned();

        // The node's own client, with nothing kept
        let args = replay_args(&dir.path().join("abci.jsonl"));
        let (_, core_answers) = open_core_rpc(&config, &args).unwrap();
        assert!(core_answers.is_none());
        assert!(!answers_path.exists());

        // A recording makes the file before the first request
        let recording = ReplayArgs {
            core_answers_record: Some(answers_path.clone()),
            ..args.clone()
        };
        let (_, core_answers) = open_core_rpc(&config, &recording).unwrap();
        let core_answers = core_answers.expect("a recording has a file");
        assert_eq!(core_answers.summary().held, 0);
        assert_eq!(
            fs::read_to_string(&answers_path).unwrap(),
            "{\"format\":\"drive-abci core answers\",\"version\":1}\n"
        );
        log_checks(&ReplayChecks::default(), Some(&core_answers));
        let checks = open_checks(&recording, None, Some(core_answers.clone())).unwrap();
        assert_eq!(checks.core_answers_failed(), Ok(()));

        // The file is then answered from, and asks no node
        let serving = ReplayArgs {
            core_answers: Some(answers_path.clone()),
            ..args.clone()
        };
        let (_, core_answers) = open_core_rpc(&config, &serving).unwrap();
        let core_answers = core_answers.expect("answers are served from a file");
        assert_eq!(core_answers.summary().held, 0);
        assert_eq!(core_answers.failure(), None);

        // A file that is not there is not served
        let missing = ReplayArgs {
            core_answers: Some(dir.path().join("missing.jsonl")),
            ..args
        };
        assert!(open_core_rpc(&config, &missing).is_err());
    }

    // ---- drain_skipped_entries ----

    #[test]
    fn drain_skipped_entries_empty_skip() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            writeln!(
                f,
                r#"{{"timestamp":"t1","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(Info(RequestInfo {{ version: \"1.0\", block_version: 14, p2p_version: 10, abci_version: \"1.0\" }})) }}"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        drain_skipped_entries(&mut stream, &[]).unwrap();
        // Item should still be available
        assert!(stream.next_item().unwrap().is_some());
    }

    #[test]
    fn drain_skipped_entries_skips_matching_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            // Line 1 - will be skipped
            writeln!(
                f,
                r#"{{"timestamp":"t1","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(Info(RequestInfo {{ version: \"1.0\", block_version: 14, p2p_version: 10, abci_version: \"1.0\" }})) }}"}}}}"#
            )
            .unwrap();
            // Line 2 - not skipped
            writeln!(
                f,
                r#"{{"timestamp":"t2","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(Info(RequestInfo {{ version: \"2.0\", block_version: 14, p2p_version: 10, abci_version: \"1.0\" }})) }}"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        let skip = vec![SkipSelector::Line(1)];
        drain_skipped_entries(&mut stream, &skip).unwrap();
        // The remaining item should be from line 2
        let item = stream.next_item().unwrap().expect("expected an item");
        assert_eq!(item.source.line(), 2);
    }

    // ---- advance_stream ----

    #[test]
    fn advance_stream_no_height_no_skip() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            writeln!(
                f,
                r#"{{"timestamp":"t1","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(Info(RequestInfo {{ version: \"1.0\", block_version: 14, p2p_version: 10, abci_version: \"1.0\" }})) }}"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        advance_stream(&mut stream, None, &[]).unwrap();
        assert!(stream.next_item().unwrap().is_some());
    }

    // ---- Log stream with Flush (filtered out) ----

    #[test]
    fn log_stream_flush_request_filtered_out() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        {
            let mut f = std::fs::File::create(&log_path).unwrap();
            writeln!(
                f,
                r#"{{"timestamp":"t1","fields":{{"message":"received ABCI request","request":"Request {{ value: Some(Flush(RequestFlush {{}})) }}"}}}}"#
            )
            .unwrap();
        }
        let mut stream = LogRequestStream::open(&log_path).unwrap();
        // Flush is filtered to None by parse_request_variant
        assert!(stream.next_item().unwrap().is_none());
    }
}
