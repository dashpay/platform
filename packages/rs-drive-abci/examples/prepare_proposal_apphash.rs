use clap::{Parser, ValueEnum};
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::platform_types::platform::Platform;
use drive_abci::rpc::core::DefaultCoreRPC;
use hex::ToHex;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tenderdash_abci::proto::abci::{request, Request, RequestPrepareProposal};
use tenderdash_abci::Application;
use tracing_subscriber::EnvFilter;

/// Replay helper for RequestPrepareProposal dumps.
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to the GroveDB database that should be used for execution.
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    db_path: PathBuf,

    /// File that contains serialized Request or RequestPrepareProposal payload.
    #[arg(long, value_hint = clap::ValueHint::FilePath)]
    request: PathBuf,

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

    let request = load_prepare_proposal(&cli.request, cli.request_format)?;

    let core_rpc = DefaultCoreRPC::open(
        config.core.consensus_rpc.url().as_str(),
        config.core.consensus_rpc.username.clone(),
        config.core.consensus_rpc.password.clone(),
    )?;

    let platform: Platform<DefaultCoreRPC> = Platform::open_with_client(
        &config.db_path,
        Some(config.clone()),
        core_rpc,
        None,
    )?;

    let app = FullAbciApplication::new(&platform);

    let response = app
        .prepare_proposal(request)
        .map_err(|err| format!("prepare_proposal failed: {:?}", err))?;

    println!("app_hash: 0x{}", response.app_hash.encode_hex::<String>());

    Ok(())
}

fn load_prepare_proposal(
    path: &Path,
    format: RequestFormat,
) -> Result<RequestPrepareProposal, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;

    match parse_with::<Request>(&raw, format) {
        Ok(request) => match request.value {
            Some(request::Value::PrepareProposal(value)) => Ok(value),
            Some(other) => Err(format!(
                "expected Request::PrepareProposal but file contains {}",
                other.variant_name()
            )
            .into()),
            None => Err("request payload does not contain a value".into()),
        },
        Err(_) => parse_with::<RequestPrepareProposal>(&raw, format),
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
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt().with_env_filter(env_filter).try_init();
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
