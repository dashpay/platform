mod replay_support;
use clap::Parser;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::platform_types::platform::Platform;
use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive_abci::rpc::core::DefaultCoreRPC;
use replay_support::cli::{Cli, SkipRequest, load_env};
use replay_support::log_ingest::LogRequestStream;
use replay_support::replay::{
    LoadedRequest, ProgressReporter, ReplayItem, ReplaySource, ensure_db_directory,
    execute_request, load_request, log_last_committed_block,
};
use replay_support::telemetry::init_logging;
use std::error::Error;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let _log_guard = init_logging(cli.output.as_deref())?;
    run(cli)
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    load_env(cli.config.as_deref())?;

    let mut config = match PlatformConfig::from_env() {
        Ok(config) => config,
        Err(drive_abci::error::Error::Configuration(envy::Error::MissingValue(field))) => {
            return Err(format!("missing configuration option: {}", field.to_uppercase()).into());
        }
        Err(err) => return Err(err.into()),
    };

    config.db_path = cli.db_path.clone();
    let db_was_created = ensure_db_directory(&config.db_path)?;

    let mut replay_items = Vec::new();
    for path in &cli.requests {
        let loaded = load_request(path, cli.request_format)?;
        tracing::info!("loaded {} request from {}", loaded.kind(), path.display());
        let canonical_path = canonicalize_path(path);
        let item = ReplayItem::from_file(canonical_path, loaded);
        if should_skip_item(&item, &cli.skip) {
            tracing::info!("skipping request {} due to --skip", item.describe());
            continue;
        }
        replay_items.push(item);
    }

    let mut log_streams = Vec::new();
    for path in &cli.logs {
        let mut stream = LogRequestStream::open(path)?;
        if stream.peek()?.is_some() {
            tracing::info!("streaming ABCI requests from log {}", path.display());
            log_streams.push(stream);
        } else {
            tracing::warn!("no supported ABCI requests found in log {}", path.display());
        }
    }

    if replay_items.is_empty() && log_streams.is_empty() {
        return Err(
            "no requests to replay; provide --requests and/or --logs with relevant inputs".into(),
        );
    }

    if db_was_created {
        let first_is_init_chain = if let Some(item) = replay_items.first() {
            matches!(item.request, LoadedRequest::InitChain(_))
        } else if let Some(stream) = log_streams.first_mut() {
            advance_stream(stream, None, &cli.skip)?;
            match stream.peek()? {
                Some(item) => matches!(item.request, LoadedRequest::InitChain(_)),
                None => false,
            }
        } else {
            false
        };

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

    if let Some(limit) = cli.stop_height {
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
    let mut progress = if cli.progress {
        Some(ProgressReporter::new(cli.stop_height))
    } else {
        None
    };

    for item in replay_items {
        if stop_height_reached(cli.stop_height, known_height) {
            tracing::info!(
                "stop height {} reached; skipping remaining request files",
                cli.stop_height.unwrap()
            );
            break;
        }
        let committed = execute_request(&app, item, progress.as_mut())?;
        update_known_height(&mut known_height, committed);
        if stop_height_reached(cli.stop_height, known_height) {
            tracing::info!(
                "stop height {} reached after request files; skipping remaining inputs",
                cli.stop_height.unwrap()
            );
            break;
        }
    }

    for mut stream in log_streams {
        if stop_height_reached(cli.stop_height, known_height) {
            tracing::info!(
                "stop height {} reached; skipping remaining log streams",
                cli.stop_height.unwrap()
            );
            break;
        }
        let mut validator = RequestSequenceValidator::new(stream.path().to_path_buf());
        let mut executed = 0usize;
        loop {
            if stop_height_reached(cli.stop_height, known_height) {
                tracing::info!(
                    "stop height {} reached; stopping replay for log {}",
                    cli.stop_height.unwrap(),
                    stream.path().display()
                );
                break;
            }
            advance_stream(&mut stream, known_height, &cli.skip)?;
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
    }

    Ok(())
}

fn update_known_height(current: &mut Option<u64>, new_height: Option<u64>) {
    if let Some(height) = new_height {
        match current {
            Some(existing) if height <= *existing => {}
            _ => *current = Some(height),
        }
    }
}

fn canonicalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn should_skip_item(item: &ReplayItem, skip: &[SkipRequest]) -> bool {
    skip.iter().any(|target| match &item.source {
        ReplaySource::File(path) => target.line.is_none() && paths_equal(path, &target.path),
        ReplaySource::Log { path, line, .. } => {
            if !paths_equal(path, &target.path) {
                return false;
            }
            match target.line {
                Some(target_line) => target_line == *line,
                None => true,
            }
        }
    })
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if let (Ok(canon_a), Ok(canon_b)) = (a.canonicalize(), b.canonicalize()) {
        canon_a == canon_b
    } else {
        a == b
    }
}

fn advance_stream(
    stream: &mut LogRequestStream,
    known_height: Option<u64>,
    skip: &[SkipRequest],
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
    skip: &[SkipRequest],
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
            Some(last) if height < last => {
                return Err(format!(
                    "log {} has out-of-order height {} before {} ({})",
                    self.path.display(),
                    last,
                    height,
                    origin
                )
                .into());
            }
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

fn stop_height_reached(limit: Option<u64>, known_height: Option<u64>) -> bool {
    matches!((limit, known_height), (Some(limit), Some(height)) if height >= limit)
}
