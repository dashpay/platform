use clap::{Parser, ValueEnum};
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::platform_types::platform::Platform;
use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive_abci::rpc::core::DefaultCoreRPC;
use hex::ToHex;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tenderdash_abci::proto::abci::{
    request,
    response_process_proposal,
    Request,
    RequestPrepareProposal,
    RequestProcessProposal,
};
use tenderdash_abci::Application;
use tracing_subscriber::EnvFilter;

/// Replay helper for RequestPrepareProposal dumps.
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to the GroveDB database that should be used for execution.
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    db_path: PathBuf,

    /// Files that contain serialized Request*, RequestPrepareProposal, or RequestProcessProposal payloads.
    /// They will be executed sequentially.
    #[arg(long, value_hint = clap::ValueHint::FilePath, required = true)]
    requests: Vec<PathBuf>,

    /// Optional .env file path. Defaults to walking up the filesystem like drive-abci.
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    config: Option<PathBuf>,

    /// Format of the serialized request payload.
    #[arg(long, value_enum, default_value_t = RequestFormat::Ron)]
    request_format: RequestFormat,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
enum RequestFormat {
    Json,
    Ron,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logging();
    let cli = Cli::parse();
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

    let mut requests = Vec::new();
    for path in &cli.requests {
        let loaded = load_request(path, cli.request_format)?;
        println!(
            "loaded {} request from {}: {:#?}",
            loaded.kind(),
            path.display(),
            loaded
        );
        requests.push((path.clone(), loaded));
    }

    if requests.is_empty() {
        return Err("no request files provided".into());
    }

    let core_rpc = DefaultCoreRPC::open(
        config.core.consensus_rpc.url().as_str(),
        config.core.consensus_rpc.username.clone(),
        config.core.consensus_rpc.password.clone(),
    )?;

    let platform: Platform<DefaultCoreRPC> =
        Platform::open_with_client(&config.db_path, Some(config.clone()), core_rpc, None)?;
    log_last_committed_block(&platform);

    let app = FullAbciApplication::new(&platform);

    for (path, request) in requests {
        match request {
            LoadedRequest::Prepare(request) => {
                println!("executing prepare_proposal from {}", path.display());
                let response = app
                    .prepare_proposal(request)
                    .map_err(|err| {
                        format!("prepare_proposal failed for {}: {:?}", path.display(), err)
                    })?;
                println!(
                    "prepare_proposal result ({}): app_hash=0x{}, tx_results={}, tx_records={}",
                    path.display(),
                    response.app_hash.encode_hex::<String>(),
                    response.tx_results.len(),
                    response.tx_records.len()
                );
            }
            LoadedRequest::Process(request) => {
                println!("executing process_proposal from {}", path.display());
                let response = app
                    .process_proposal(request)
                    .map_err(|err| {
                        format!("process_proposal failed for {}: {:?}", path.display(), err)
                    })?;
                let status = response_process_proposal::ProposalStatus::from_i32(response.status)
                    .unwrap_or(response_process_proposal::ProposalStatus::Unknown);
                println!(
                    "process_proposal result ({}): status={:?}, app_hash=0x{}, tx_results={}, events={}",
                    path.display(),
                    status,
                    hex::encode(response.app_hash),
                    response.tx_results.len(),
                    response.events.len()
                );
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
enum LoadedRequest {
    Prepare(RequestPrepareProposal),
    Process(RequestProcessProposal),
}

impl LoadedRequest {
    fn kind(&self) -> &'static str {
        match self {
            LoadedRequest::Prepare(_) => "prepare_proposal",
            LoadedRequest::Process(_) => "process_proposal",
        }
    }
}

fn load_request(path: &Path, format: RequestFormat) -> Result<LoadedRequest, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;

    if let Ok(request) = parse_with::<Request>(&raw, format) {
        return match request.value {
            Some(request::Value::PrepareProposal(value)) => Ok(LoadedRequest::Prepare(value)),
            Some(request::Value::ProcessProposal(value)) => Ok(LoadedRequest::Process(value)),
            Some(other) => Err(format!(
                "expected Request::PrepareProposal or Request::ProcessProposal but file contains {}",
                other.variant_name()
            )
            .into()),
            None => Err("request payload does not contain a value".into()),
        };
    }

    parse_with::<RequestPrepareProposal>(&raw, format)
        .map(LoadedRequest::Prepare)
        .or_else(|_| {
            parse_with::<RequestProcessProposal>(&raw, format).map(LoadedRequest::Process)
        })
}

fn log_last_committed_block<C>(platform: &Platform<C>)
where
    C: drive_abci::rpc::core::CoreRPCLike,
{
    let platform_state = platform.state.load();
    if let Some(info) = platform_state.last_committed_block_info() {
        let basic_info = info.basic_info();
        println!(
            "last_committed_block: height={}, round={}, core_height={}, block_id_hash=0x{}",
            basic_info.height,
            info.round(),
            basic_info.core_height,
            hex::encode(info.block_id_hash())
        );
    } else {
        println!("last_committed_block: None");
    }
}

fn load_env(path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    if let Some(path) = path {
        dotenvy::from_path(path)?;
        return Ok(());
    }

    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(err) if err.not_found() => {
            eprintln!("warning: no .env file found");
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

fn parse_with<T>(raw: &str, format: RequestFormat) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    match format {
        RequestFormat::Json => Ok(serde_json::from_str(raw)?),
        RequestFormat::Ron => Ok(ron::from_str(raw)?),
    }
}

fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();
}
trait RequestVariantName {
    fn variant_name(&self) -> &'static str;
}

impl RequestVariantName for request::Value {
    fn variant_name(&self) -> &'static str {
        match self {
            request::Value::Echo(_) => "Echo",
            request::Value::Flush(_) => "Flush",
            request::Value::Info(_) => "Info",
            request::Value::InitChain(_) => "InitChain",
            request::Value::Query(_) => "Query",
            request::Value::CheckTx(_) => "CheckTx",
            request::Value::ListSnapshots(_) => "ListSnapshots",
            request::Value::OfferSnapshot(_) => "OfferSnapshot",
            request::Value::LoadSnapshotChunk(_) => "LoadSnapshotChunk",
            request::Value::ApplySnapshotChunk(_) => "ApplySnapshotChunk",
            request::Value::PrepareProposal(_) => "PrepareProposal",
            request::Value::ProcessProposal(_) => "ProcessProposal",
            request::Value::ExtendVote(_) => "ExtendVote",
            request::Value::VerifyVoteExtension(_) => "VerifyVoteExtension",
            request::Value::FinalizeBlock(_) => "FinalizeBlock",
        }
    }
}
