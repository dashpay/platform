use clap::{Parser, ValueEnum};
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use drive_abci::abci::app::FullAbciApplication;
use drive_abci::config::{FromEnv, PlatformConfig};
use drive_abci::platform_types::platform::Platform;
use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive_abci::rpc::core::DefaultCoreRPC;
use hex::ToHex;
use serde::de::DeserializeOwned;
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use tenderdash_abci::Application;
use tenderdash_abci::proto::abci::{
    Request, RequestPrepareProposal, RequestProcessProposal, request, response_process_proposal,
};
use textwrap::Options;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::registry::LookupSpan;

const TRACE_WRAP_WIDTH: usize = 120;

/// Replay helper for RequestPrepareProposal dumps.
#[derive(Debug, Parser)]
#[command(
    name = "replay_abci_requests",
    author,
    version,
    about = "Replay serialized ABCI requests against an existing GroveDB database.",
    long_about = "Feed captured RequestPrepareProposal or RequestProcessProposal payloads (RON or JSON) \
sequentially into the Drive ABCI application to recompute app hashes, inspect tx outcomes, and debug \
state mismatches. Request files accept both the outer Request wrapper or the specific request type, \
and configuration mirrors drive-abci's .env loading so you can point at the same RPC credentials. \
\n\nExample:\n  replay_abci_requests --db-path /path/to/grovedb --requests dump.ron \
--config /path/to/.env --request-format ron\n\nUse multiple --requests flags to replay several inputs \
in chronological order."
)]
struct Cli {
    /// Path to the GroveDB database that should be used for execution.
    /// You can use a command like `./state_backup.sh export --component abci abci.tar.gz testnet`
    /// to dump the GroveDB database from existing Platform node.
    #[arg(long, value_hint = clap::ValueHint::DirPath)]
    db_path: PathBuf,

    /// Files that contain serialized Request*, RequestPrepareProposal, or RequestProcessProposal payloads.
    /// They will be executed sequentially.
    ///
    /// See vectors/ directory for example request payloads.
    #[arg(long, value_hint = clap::ValueHint::FilePath, required = true)]
    requests: Vec<PathBuf>,

    /// Optional .env file path. Defaults to walking up the filesystem like drive-abci.
    /// .env file format is the same as used by drive-abci.
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
        tracing::info!(
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
                let height = request.height;
                tracing::info!("executing prepare_proposal from {}", path.display());
                let response = app.prepare_proposal(request).map_err(|err| {
                    format!("prepare_proposal failed for {}: {:?}", path.display(), err)
                })?;
                tracing::info!(
                    "prepare_proposal result ({}): height={}, app_hash=0x{}, tx_results={}, tx_records={}",
                    path.display(),
                    height,
                    response.app_hash.encode_hex::<String>(),
                    response.tx_results.len(),
                    response.tx_records.len()
                );
            }
            LoadedRequest::Process(request) => {
                tracing::info!("executing process_proposal from {}", path.display());
                let height = request.height;
                let response = app.process_proposal(request).map_err(|err| {
                    format!("process_proposal failed for {}: {:?}", path.display(), err)
                })?;
                let status = response_process_proposal::ProposalStatus::try_from(response.status)
                    .unwrap_or(response_process_proposal::ProposalStatus::Unknown);
                tracing::info!(
                    "process_proposal result ({}): status={:?}, height={}, app_hash=0x{}, tx_results={}, events={}",
                    path.display(),
                    status,
                    height,
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
        .or_else(|_| parse_with::<RequestProcessProposal>(&raw, format).map(LoadedRequest::Process))
}

fn log_last_committed_block<C>(platform: &Platform<C>)
where
    C: drive_abci::rpc::core::CoreRPCLike,
{
    let platform_state = platform.state.load();
    if let Some(info) = platform_state.last_committed_block_info() {
        let basic_info = info.basic_info();
        tracing::info!(
            "last_committed_block: height={}, round={}, core_height={}, block_id_hash=0x{}",
            basic_info.height,
            info.round(),
            basic_info.core_height,
            hex::encode(info.block_id_hash())
        );
    } else {
        tracing::info!("last_committed_block: None");
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
            tracing::warn!("warning: no .env file found");
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
    let json_format = PrettyJsonEventFormat::new(TRACE_WRAP_WIDTH);

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .event_format(json_format)
        .try_init();
}

struct PrettyJsonEventFormat {
    width: usize,
}

impl PrettyJsonEventFormat {
    fn new(width: usize) -> Self {
        Self { width }
    }
}

impl<S, N> FormatEvent<S, N> for PrettyJsonEventFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut field_visitor = JsonFieldsVisitor::default();
        event.record(&mut field_visitor);
        let fields = field_visitor.finish();

        let mut payload = JsonMap::new();
        payload.insert(
            "level".into(),
            JsonValue::String(event.metadata().level().to_string()),
        );
        payload.insert(
            "target".into(),
            JsonValue::String(event.metadata().target().to_string()),
        );

        if let Some(file) = event.metadata().file() {
            payload.insert("file".into(), JsonValue::String(file.to_string()));
        }

        if let Some(line) = event.metadata().line() {
            payload.insert("line".into(), JsonValue::Number(JsonNumber::from(line)));
        }

        if !fields.is_empty() {
            payload.insert("fields".into(), JsonValue::Object(fields));
        }

        let spans = collect_span_names(ctx);
        if !spans.is_empty() {
            payload.insert(
                "spans".into(),
                JsonValue::Array(spans.into_iter().map(JsonValue::String).collect()),
            );
        }

        let json =
            serde_json::to_string_pretty(&JsonValue::Object(payload)).map_err(|_| fmt::Error)?;

        write_wrapped(writer, &json, self.width)
    }
}

fn write_wrapped(mut writer: Writer<'_>, text: &str, width: usize) -> fmt::Result {
    if text.is_empty() {
        writer.write_char('\n')?;
        return Ok(());
    }

    let mut remaining = text;

    while let Some(idx) = remaining.find('\n') {
        let line = &remaining[..idx];
        emit_wrapped_line(&mut writer, line, width)?;
        remaining = &remaining[idx + 1..];
    }

    if !remaining.is_empty() {
        emit_wrapped_line(&mut writer, remaining, width)?;
    }

    Ok(())
}

fn emit_wrapped_line(writer: &mut Writer<'_>, line: &str, width: usize) -> fmt::Result {
    if line.is_empty() {
        writer.write_char('\n')?;
        return Ok(());
    }

    let indent_end = line
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| line.len());
    let (indent, content) = line.split_at(indent_end);

    if content.is_empty() {
        writer.write_str(indent)?;
        writer.write_char('\n')?;
        return Ok(());
    }

    let indent_chars = indent.chars().count();
    let mut effective_width = width.saturating_sub(indent_chars);
    if effective_width == 0 {
        effective_width = 1;
    }

    let wrap_options = Options::new(effective_width).break_words(false);
    let wrapped = textwrap::wrap(content, &wrap_options);
    for piece in wrapped {
        if !indent.is_empty() {
            writer.write_str(indent)?;
        }
        writer.write_str(piece.as_ref())?;
        writer.write_char('\n')?;
    }

    Ok(())
}

fn collect_span_names<S, N>(ctx: &FmtContext<'_, S, N>) -> Vec<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    let mut names = Vec::new();
    let mut current = ctx.lookup_current();
    while let Some(span) = current {
        names.push(span.name().to_string());
        current = span.parent();
    }
    names.reverse();
    names
}

#[derive(Default)]
struct JsonFieldsVisitor {
    fields: JsonMap<String, JsonValue>,
}

impl JsonFieldsVisitor {
    fn finish(self) -> JsonMap<String, JsonValue> {
        self.fields
    }

    fn insert_value(&mut self, field: &Field, value: JsonValue) {
        self.fields.insert(field.name().to_string(), value);
    }
}

impl Visit for JsonFieldsVisitor {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert_value(field, JsonValue::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert_value(field, JsonValue::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert_value(field, JsonValue::Number(value.into()));
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.insert_value(field, JsonValue::String(value.to_string()));
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.insert_value(field, JsonValue::String(value.to_string()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        let json_value = JsonNumber::from_f64(value)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null);
        self.insert_value(field, json_value);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert_value(field, JsonValue::String(value.to_owned()));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.insert_value(field, JsonValue::String(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.insert_value(field, JsonValue::String(format!("{value:?}")));
    }

    fn record_bytes(&mut self, field: &Field, value: &[u8]) {
        self.insert_value(field, JsonValue::String(hex::encode(value)));
    }
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
