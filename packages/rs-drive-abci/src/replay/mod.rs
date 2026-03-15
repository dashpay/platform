mod cli;
mod log_ingest;
mod runner;

use crate::abci::app::FullAbciApplication;
use crate::config::PlatformConfig;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::DefaultCoreRPC;
use crate::verify;
use cli::SkipSelector;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use log_ingest::LogRequestStream;
use runner::{
    ensure_db_directory, execute_request, log_last_committed_block, stop_height_reached,
    LoadedRequest, ProgressReporter, ReplayItem,
};
use std::error::Error;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

pub use cli::ReplayArgs;

/// Replay ABCI requests captured in drive-abci JSON logs.
pub fn run(
    mut config: PlatformConfig,
    args: ReplayArgs,
    cancel: CancellationToken,
) -> Result<(), Box<dyn Error>> {
    let db_path = args
        .db_path
        .clone()
        .unwrap_or_else(|| config.db_path.clone());
    config.db_path = db_path;
    let db_was_created = ensure_db_directory(&config.db_path)?;

    tracing::info!("running database verification before replay");
    verify::run(&config, true).map_err(|e| format!("verification failed before replay: {}", e))?;

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

    if db_was_created {
        let mut first_is_init_chain = false;
        advance_stream(&mut stream, None, &args.skip)?;
        if let Some(item) = stream.peek()? {
            first_is_init_chain = matches!(item.request, LoadedRequest::InitChain(_));
        }

        if !first_is_init_chain {
            return Err(
                "database path did not exist; first replayed request must be init_chain".into(),
            );
        }
    }

    let core_rpc = DefaultCoreRPC::open(
        config.core.consensus_rpc.url().as_str(),
        config.core.consensus_rpc.username.clone(),
        config.core.consensus_rpc.password.clone(),
    )?;

    let platform: Platform<DefaultCoreRPC> =
        Platform::open_with_client(&config.db_path, Some(config.clone()), core_rpc, None)?;
    log_last_committed_block(&platform);
    let mut known_height = platform
        .state
        .load()
        .last_committed_block_info()
        .as_ref()
        .map(|info| info.basic_info().height);

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
        if stop_height_reached(args.stop_height, known_height) {
            tracing::info!(
                "stop height {} reached; stopping replay for log {}",
                args.stop_height.unwrap(),
                stream.path().display()
            );
            break;
        }
        advance_stream(&mut stream, known_height, &args.skip)?;
        let Some(item) = stream.next_item()? else {
            break;
        };
        validator.observe(&item)?;
        let committed = execute_request(&app, item, progress.as_mut())?;
        update_known_height(&mut known_height, committed);
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
        if self.last_height.is_some() && (!self.saw_process || !self.saw_finalize) {
            return Err(format!(
                "log {} ended before height {} had both process_proposal and finalize_block",
                self.path.display(),
                self.last_height.unwrap()
            )
            .into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runner::LoadedRequest;
    use std::io::Write;
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
