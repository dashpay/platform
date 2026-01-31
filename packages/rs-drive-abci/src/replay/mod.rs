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
