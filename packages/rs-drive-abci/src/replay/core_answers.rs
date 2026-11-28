//! Dash Core's answers to a replay, kept in a file.
//!
//! Block execution asks Core over JSON-RPC, and the recorded ABCI requests do not hold what
//! Core answered. A replay that records runs beside a Core node and keeps what Core answered
//! to each request in a file; a replay given that file answers from it and needs no Core node.
//!
//! Both sit under [`DefaultCoreRPC`] as the JSON-RPC transport of its client, so the calls and
//! their decoding are the node's own, and what is kept is Core's JSON as it came.
//!
//! The file is JSON lines. The first names the format. Every other line is one request and
//! its answer: a `result`, an `error`, or neither where Core answered `null`:
//!
//! ```text
//! {"format":"drive-abci core answers","version":1}
//! {"method":"getblockhash","params":[1090320],"result":"000000000000001b…"}
//! {"method":"getrawtransaction","params":["5d1b…",1],"error":{"code":-5,"message":"…","data":null}}
//! ```
//!
//! A request is its method and its parameters, and the file holds one answer for it: the
//! first Core gave that is kept. Not kept is what is no answer to the request: an error of the
//! connection, and Core's errors that say it is not ready (starting, not connected, loading
//! blocks), after which the recording asks again. A request the file holds an answer to is,
//! when asked again, asked of Core again and not written again.
//!
//! A node's client asks again after such an error too, but not for ever, and some of block
//! execution goes on when it has given up (it hands Core a withdrawal transaction and notes
//! a failure; a proposer goes on without a chain lock). What it was given then is not in the
//! file, and a replay from the file would be given something else. So a recording does not
//! hand such an outcome up to the client. Where Core gives no answer, the recording itself
//! asks again, as often and with the waits the node's client has, and the client sees
//! nothing of it when an answer comes. When Core has given none to any of the tries, the
//! run has failed, and one of two things follows:
//!
//! - Outside `FinalizeBlock` the request ends in an error of the transport that says so, and
//!   so does every later request. The client and block execution may make of that error
//!   what they will until the ABCI request has ended, where the replay ends. No block is
//!   committed outside `FinalizeBlock`.
//! - `FinalizeBlock` commits the block before it ends. While it is executed, a failure of
//!   Core's answers is therefore not given to the client at all: it ends the request at
//!   once, before the block's transaction is committed. See [`CoreAnswers::finalizing`],
//!   which also says when the replay can be started again on the same database.
//!
//! Answers are served by request and not by their order, so a replay may start at any height
//! the recording covered. This rests on Core answering a request alike whenever it is asked.
//! A recording therefore wants a Core whose chain stands still: a node that has caught up
//! with its network and then had its networking switched off (`setnetworkactive false`). A
//! node that was never connected has no chain lock to answer with. A chain that stands still
//! does not make every answer the same (an answer may hold something that follows the
//! clock), so it is checked: where Core does answer a recorded request differently, with an
//! answer that would be kept, the recording stops there. The file keeps the first answer,
//! and the replay ends with an error.
//!
//! Block execution also hands Core chain locks and withdrawal transactions (`submitchainlock`,
//! `sendrawtransaction`). They are requests like the others here: asked of Core by a
//! recording, which is one more reason for its networking to be off, and answered from the
//! file otherwise. Core's answer to them depends on what it already holds, so a recording
//! that hands Core a transaction a second time can meet a different answer and stop.
//!
//! A recording needs every method block execution asks for allowed to its RPC user. The
//! node's client reads a refusal by HTTP status 403 (a method missing from `rpcwhitelist`) as
//! a reason to ask another way; a file holds answers, not refusals, so a replay from it could
//! not take the same way. A recording therefore stops at such a refusal.

use crate::rpc::core::{
    DefaultCoreRPC, CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD, CORE_RPC_CLIENT_NOT_CONNECTED,
    CORE_RPC_ERROR_IN_WARMUP,
};
use dpp::dashcore::hashes::sha256;
use dpp::dashcore_rpc::jsonrpc::error::RpcError;
use dpp::dashcore_rpc::jsonrpc::simple_http::Error as HttpError;
use dpp::dashcore_rpc::jsonrpc::{self, Request, Response, Transport};
use dpp::dashcore_rpc::{Auth, Client};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::str;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ThreadId};
use std::time::Duration;

/// The HTTP status Core refuses a request with, such as a method missing from the RPC user's
/// `rpcwhitelist`
const HTTP_FORBIDDEN: u16 = 403;

const FORMAT: &str = "drive-abci core answers";
const VERSION: u32 = 1;

/// How often a recording asks Core a request before the run fails: as often as the node's
/// client asks (`retry!` in `rpc/core.rs`).
const TRIES: u32 = 4;

/// The wait of a recording before it asks Core a second time. It waits twice as long before
/// the third time and three times as long before the fourth, as the node's client does.
const WAIT: Duration = Duration::from_secs(1);

/// Errors of Core that say the node is not ready. They are not answers to the request, and
/// the node's own client asks again after them, so a recording does not keep them and asks
/// again itself.
const NOT_READY: [i32; 3] = [
    CORE_RPC_ERROR_IN_WARMUP,
    CORE_RPC_CLIENT_NOT_CONNECTED,
    CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD,
];

type Digest = [u8; 32];

#[derive(Debug, Serialize, Deserialize)]
struct Header {
    format: String,
    version: u32,
}

/// One line of the file: a request and Core's answer to it.
#[derive(Debug, Serialize, Deserialize)]
struct Exchange {
    method: String,
    params: Box<RawValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<Box<RawValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

/// The text of a request's parameters; a request without any reads as JSON `null`.
fn params_text(params: Option<&RawValue>) -> &str {
    params.map(RawValue::get).unwrap_or("null")
}

/// JSON on one line. Core answers on one line; an answer that is not is written without its
/// line breaks, since a line of the file ends where the answer ends. Nothing else of the text
/// changes: a line break in JSON is either between two tokens, where it means nothing, or
/// inside a string, where JSON does not allow one unescaped.
fn on_one_line(text: &str) -> Result<Box<RawValue>, serde_json::Error> {
    if !text.contains(['\n', '\r']) {
        return RawValue::from_string(text.to_owned());
    }
    let mut one_line = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        if in_string {
            in_string = escaped || character != '"';
            escaped = !escaped && character == '\\';
        } else if character == '"' {
            in_string = true;
        } else if character == '\n' || character == '\r' {
            continue;
        }
        one_line.push(character);
    }
    RawValue::from_string(one_line)
}

fn request_digest(method: &str, params: &str) -> Digest {
    let mut bytes = Vec::with_capacity(method.len() + 1 + params.len());
    bytes.extend_from_slice(method.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(params.as_bytes());
    sha256::Hash::hash(&bytes).to_byte_array()
}

impl Exchange {
    fn new(method: &str, params: &str, response: &Response) -> Result<Self, serde_json::Error> {
        Ok(Exchange {
            method: method.to_owned(),
            params: on_one_line(params)?,
            result: response
                .result
                .as_deref()
                .map(|result| on_one_line(result.get()))
                .transpose()?,
            error: response
                .error
                .as_ref()
                .map(|error| {
                    Ok::<_, serde_json::Error>(RpcError {
                        code: error.code,
                        message: error.message.clone(),
                        data: error
                            .data
                            .as_deref()
                            .map(|data| on_one_line(data.get()))
                            .transpose()?,
                    })
                })
                .transpose()?,
        })
    }

    fn request_digest(&self) -> Digest {
        request_digest(&self.method, self.params.get())
    }

    fn answer_digest(&self) -> Result<Digest, serde_json::Error> {
        let mut bytes = Vec::new();
        if let Some(result) = &self.result {
            bytes.extend_from_slice(result.get().as_bytes());
        }
        bytes.push(0);
        if let Some(error) = &self.error {
            serde_json::to_writer(&mut bytes, error)?;
        }
        Ok(sha256::Hash::hash(&bytes).to_byte_array())
    }

    fn describe(&self) -> String {
        format!("`{}` {}", self.method, self.params.get())
    }
}

/// Where the answer to a request is in the file, and what it is.
#[derive(Debug, Clone, Copy)]
struct Held {
    offset: u64,
    answer: Digest,
}

#[derive(Debug)]
struct State {
    file: File,
    /// Where the next line goes
    end: u64,
    held: HashMap<Digest, Held>,
    /// Requests of this run, by method: those Core gave an answer to keep to, or those
    /// looked up in the file
    asked: BTreeMap<String, u64>,
    /// Answers this run added to the file
    added: u64,
    /// Why this run can no longer be trusted; every later request is refused with it
    failure: Option<String>,
    /// The thread that executes `FinalizeBlock`, while it does: a failure found on it unwinds
    /// (`CoreAnswers::finalizing`)
    finalizing: Option<ThreadId>,
}

/// What unwinds out of `FinalizeBlock` when Core's answers fail while it is executed. The
/// reason is the run's failure.
struct EndedBeforeCommit;

#[derive(Debug)]
struct Shared {
    path: PathBuf,
    state: Mutex<State>,
}

#[derive(Debug)]
struct CoreAnswersError(String);

impl fmt::Display for CoreAnswersError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CoreAnswersError {}

fn transport_error(message: String) -> jsonrpc::Error {
    jsonrpc::Error::Transport(Box::new(CoreAnswersError(message)))
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Notes why the run is no longer to be trusted and gives the error for the request.
    /// While `FinalizeBlock` is executed, on its thread, there is no error to give: the
    /// failure unwinds to `CoreAnswers::finalizing`.
    fn fail(&self, state: &mut State, message: String) -> jsonrpc::Error {
        tracing::error!("{}", message);
        if state.failure.is_none() {
            state.failure = Some(message.clone());
        }
        if state.finalizing == Some(thread::current().id()) {
            // Without the panic hook: this is no panic, and the reason is logged above
            panic::resume_unwind(Box::new(EndedBeforeCommit));
        }
        transport_error(message)
    }

    fn refuse_after_failure(&self) -> Result<(), jsonrpc::Error> {
        match &self.lock().failure {
            Some(failure) => Err(transport_error(failure.clone())),
            None => Ok(()),
        }
    }

    fn record(
        &self,
        method: &str,
        params: &str,
        response: &Response,
    ) -> Result<(), jsonrpc::Error> {
        if not_ready(response).is_some() {
            return Ok(());
        }

        let mut state = self.lock();
        *state.asked.entry(method.to_owned()).or_default() += 1;

        let digests = Exchange::new(method, params, response).and_then(|exchange| {
            let answer = exchange.answer_digest()?;
            Ok((exchange, answer))
        });
        let (exchange, answer) = match digests {
            Ok(digests) => digests,
            Err(error) => {
                let message = format!(
                    "Core's answer to `{}` {} cannot be written to {}: {}",
                    method,
                    params,
                    self.path.display(),
                    error
                );
                return Err(self.fail(&mut state, message));
            }
        };

        let held = state
            .held
            .get(&exchange.request_digest())
            .map(|held| held.answer);
        match held {
            Some(held) if held == answer => return Ok(()),
            Some(_) => {
                let message = format!(
                    "Core answered {} differently from the answer {} holds for it; a recording needs a Core whose chain stands still",
                    exchange.describe(),
                    self.path.display()
                );
                return Err(self.fail(&mut state, message));
            }
            None => {}
        }

        let written = serde_json::to_vec(&exchange)
            .map_err(io::Error::from)
            .and_then(|mut line| {
                line.push(b'\n');
                state.file.write_all(&line)?;
                Ok(line.len() as u64)
            });
        match written {
            Ok(length) => {
                let offset = state.end;
                state
                    .held
                    .insert(exchange.request_digest(), Held { offset, answer });
                state.end += length;
                state.added += 1;
                Ok(())
            }
            Err(error) => {
                let message = format!(
                    "Core's answer to {} cannot be written to {}: {}",
                    exchange.describe(),
                    self.path.display(),
                    error
                );
                Err(self.fail(&mut state, message))
            }
        }
    }

    fn answer(&self, request: &Request) -> Result<Response, jsonrpc::Error> {
        let params = params_text(request.params);
        let mut state = self.lock();
        if let Some(failure) = &state.failure {
            return Err(transport_error(failure.clone()));
        }
        *state.asked.entry(request.method.to_owned()).or_default() += 1;

        let Some(held) = state
            .held
            .get(&request_digest(request.method, params))
            .copied()
        else {
            let message = format!(
                "{} holds no answer of Core to `{}` {}",
                self.path.display(),
                request.method,
                params
            );
            return Err(self.fail(&mut state, message));
        };

        // The line is read again at every request, and the file may have been changed under
        // the replay since it was read whole: what is served has to be the answer that was
        // found there then, to this request.
        let exchange = read_line_at(&state.file, held.offset).and_then(|line| {
            let exchange = serde_json::from_str::<Exchange>(&line)?;
            if exchange.request_digest() != request_digest(request.method, params)
                || exchange.answer_digest()? != held.answer
            {
                return Err(io::Error::other(
                    "the file was changed after the replay had read it",
                ));
            }
            Ok(exchange)
        });
        match exchange {
            Ok(exchange) => Ok(Response {
                result: exchange.result,
                error: exchange.error,
                id: request.id.clone(),
                jsonrpc: request.jsonrpc.map(str::to_owned),
            }),
            Err(error) => {
                let message = format!(
                    "the answer of Core to `{}` {} cannot be read from {}: {}",
                    request.method,
                    params,
                    self.path.display(),
                    error
                );
                Err(self.fail(&mut state, message))
            }
        }
    }
}

fn read_line_at(mut file: &File, offset: u64) -> io::Result<String> {
    file.seek(SeekFrom::Start(offset))?;
    let mut line = String::new();
    BufReader::new(file).read_line(&mut line)?;
    Ok(line)
}

/// What a file holds: the answer to each request, and where the file's whole lines end.
struct Loaded {
    held: HashMap<Digest, Held>,
    end: u64,
    /// Whether bytes follow the last whole line: a line that was being written when its
    /// writer was stopped
    torn: bool,
}

fn load(file: &File, path: &Path) -> Result<Loaded, String> {
    let mut reader = BufReader::new(file);
    let mut held = HashMap::new();
    let mut end = 0u64;
    let mut bytes = Vec::new();
    let mut number = 0usize;
    loop {
        // Read as bytes: a line that was being written when its writer was stopped can end
        // inside a character, and is told by its missing line end before anything is decoded
        bytes.clear();
        let length = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| format!("cannot read {}: {}", path.display(), error))?;
        if length == 0 || !bytes.ends_with(b"\n") {
            if number == 0 {
                return Err(format!(
                    "{} is not a file of Core's answers: it has no format line",
                    path.display()
                ));
            }
            return Ok(Loaded {
                held,
                end,
                torn: length != 0,
            });
        }
        number += 1;
        let line = str::from_utf8(&bytes).map_err(|error| {
            format!(
                "line {} of {} is not text: {}",
                number,
                path.display(),
                error
            )
        })?;

        if number == 1 {
            let header: Header = serde_json::from_str(line).map_err(|error| {
                format!(
                    "{} does not start with the line of a file of Core's answers: {}",
                    path.display(),
                    error
                )
            })?;
            if header.format != FORMAT || header.version != VERSION {
                return Err(format!(
                    "{} is `{}` version {}; this replay reads `{}` version {}",
                    path.display(),
                    header.format,
                    header.version,
                    FORMAT,
                    VERSION
                ));
            }
        } else {
            let exchange: Exchange = serde_json::from_str(line).map_err(|error| {
                format!(
                    "line {} of {} is not a request and its answer: {}",
                    number,
                    path.display(),
                    error
                )
            })?;
            let answer = exchange
                .answer_digest()
                .map_err(|error| format!("line {} of {}: {}", number, path.display(), error))?;
            let earlier = held.insert(
                exchange.request_digest(),
                Held {
                    offset: end,
                    answer,
                },
            );
            if earlier.is_some() {
                return Err(format!(
                    "line {} of {} answers {}, which an earlier line answers too",
                    number,
                    path.display(),
                    exchange.describe()
                ));
            }
        }
        end += length as u64;
    }
}

/// Whether Core refused the request with HTTP 403 and no JSON-RPC answer
fn is_http_forbidden(error: &jsonrpc::Error) -> bool {
    let jsonrpc::Error::Transport(transport) = error else {
        return false;
    };
    matches!(
        transport.downcast_ref::<HttpError>(),
        Some(HttpError::HttpErrorCode(HTTP_FORBIDDEN))
    )
}

/// The node's own client to a Core node, as what a recording asks through: made as the node
/// makes it, so what the node refuses of an address, a recording refuses too.
struct NodeClient(Client);

impl Transport for NodeClient {
    fn send_request(&self, request: Request) -> Result<Response, jsonrpc::Error> {
        self.0.get_jsonrpc_client().send_request(request)
    }

    fn send_batch(&self, _: &[Request]) -> Result<Vec<Response>, jsonrpc::Error> {
        Err(transport_error(
            "a batch of requests to Core is not recorded".to_owned(),
        ))
    }

    fn fmt_target(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0.get_jsonrpc_client())
    }
}

/// Core's error that says the node is not ready, if the response is one: no answer to the
/// request.
fn not_ready(response: &Response) -> Option<&RpcError> {
    response
        .error
        .as_ref()
        .filter(|error| NOT_READY.contains(&error.code))
}

/// The transport of a recording replay: asks Core, and keeps what it answered.
///
/// What it gives the client is an answer of Core, which the file then holds, or, once the
/// run has failed, an error that says why (while `FinalizeBlock` is executed not even that:
/// see `Shared::fail`). Where Core gives no answer (an error of the connection, or its error
/// for a node that is not ready) the request is asked again here, `TRIES` times in all, and
/// not handed up for the client to ask again: when the client gives up, what asked through
/// it may go on, and nothing in the file would say that it did so without an answer. Those
/// are the errors the node's client asks again after. After any other error of a request
/// (a reply with HTTP status 200 whose body is no JSON, for one; with another status it is
/// an error of the connection) the node's client does not, so the recording asks no more and
/// has failed: an answer to a second try would be one the node had not been given.
struct Recording<T> {
    core: T,
    shared: Arc<Shared>,
    /// The wait before the second try; see `WAIT`
    wait: Duration,
}

impl<T: Transport> Transport for Recording<T> {
    fn send_request(&self, request: Request) -> Result<Response, jsonrpc::Error> {
        self.shared.refuse_after_failure()?;
        let method = request.method.to_owned();
        let params = params_text(request.params).to_owned();
        let mut tries = 0;
        loop {
            tries += 1;
            let instead = match self.core.send_request(request.clone()) {
                Ok(response) => match not_ready(&response) {
                    None => {
                        self.shared.record(&method, &params, &response)?;
                        return Ok(response);
                    }
                    Some(error) => format!("its error {} ({})", error.code, error.message),
                },
                Err(error) if is_http_forbidden(&error) => {
                    let message = format!(
                        "Core refused `{}` with HTTP status 403; a recording needs every method block execution asks for allowed to its RPC user, since {} holds answers and no refusals",
                        method,
                        self.shared.path.display()
                    );
                    return Err(self.shared.fail(&mut self.shared.lock(), message));
                }
                // What the node's client asks again after: an error of the connection
                Err(error @ jsonrpc::Error::Transport(_)) => error.to_string(),
                // Any other error the node's client hands to block execution at once, and no
                // later answer may stand in the file for it
                Err(error) => {
                    let message = format!(
                        "Core's reply to `{}` {} is no answer: {}; the node's client does not ask again after such an error, and the recording ends here, since {} holds answers of Core and nothing else",
                        method,
                        params,
                        error,
                        self.shared.path.display()
                    );
                    return Err(self.shared.fail(&mut self.shared.lock(), message));
                }
            };
            if tries == TRIES {
                let message = format!(
                    "Core gave no answer to `{}` {} in {} tries, the last time: {}; the recording ends here, since {} holds answers of Core and nothing else",
                    method,
                    params,
                    TRIES,
                    instead,
                    self.shared.path.display()
                );
                let mut state = self.shared.lock();
                return Err(self.shared.fail(&mut state, message));
            }
            tracing::warn!(
                "Core gave no answer to `{}` (try {} of {}): {}",
                method,
                tries,
                TRIES,
                instead
            );
            thread::sleep(self.wait * tries);
        }
    }

    fn send_batch(&self, _: &[Request]) -> Result<Vec<Response>, jsonrpc::Error> {
        Err(transport_error(
            "a batch of requests to Core is not recorded".to_owned(),
        ))
    }

    fn fmt_target(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.core.fmt_target(f)?;
        write!(f, ", recorded in {}", self.shared.path.display())
    }
}

/// The transport of a replay without Core: answers from the file.
struct Serving {
    shared: Arc<Shared>,
}

impl Transport for Serving {
    fn send_request(&self, request: Request) -> Result<Response, jsonrpc::Error> {
        self.shared.answer(&request)
    }

    fn send_batch(&self, _: &[Request]) -> Result<Vec<Response>, jsonrpc::Error> {
        Err(transport_error(
            "a batch of requests to Core is not answered from a file".to_owned(),
        ))
    }

    fn fmt_target(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Core's answers in {}", self.shared.path.display())
    }
}

/// What a run asked Core and what its file holds, for the replay's last lines.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct CoreAnswersSummary {
    /// Requests of this run, by method. Beside a Core node: those Core gave an answer to
    /// keep to; one that Core answered with an error for a node that is not ready, or that
    /// did not reach Core, is not counted until it is asked again and answered. Without one:
    /// those looked up in the file, the one that was not found and ended the replay among
    /// them
    pub(super) asked: BTreeMap<String, u64>,
    /// Requests the file holds an answer to
    pub(super) held: u64,
    /// Answers this run added to the file
    pub(super) added: u64,
}

/// The replay's hold on the file of Core's answers, beside the client that asks through it.
#[derive(Debug, Clone)]
pub(super) struct CoreAnswers {
    shared: Arc<Shared>,
}

impl CoreAnswers {
    /// A client that asks the Core node at `url` and keeps its answers in the file at `path`.
    /// A file that exists is added to: the requests it holds are not written again, and
    /// bytes after its last whole line, left by a recording that was stopped, are cut off.
    pub(super) fn record(
        path: &Path,
        url: &str,
        username: String,
        password: String,
    ) -> Result<(Self, DefaultCoreRPC), Box<dyn Error>> {
        let core = NodeClient(Client::new(url, Auth::UserPass(username, password))?);
        let (answers, client) = Self::recording(path, core, WAIT)?;
        Ok((
            answers,
            DefaultCoreRPC::with_client(Client::from_jsonrpc(client)),
        ))
    }

    /// A client that answers from the file at `path` and asks no Core node.
    pub(super) fn serve(path: &Path) -> Result<(Self, DefaultCoreRPC), Box<dyn Error>> {
        let (answers, client) = Self::serving(path)?;
        Ok((
            answers,
            DefaultCoreRPC::with_client(Client::from_jsonrpc(client)),
        ))
    }

    fn recording<T: Transport>(
        path: &Path,
        core: T,
        wait: Duration,
    ) -> Result<(Self, jsonrpc::Client), Box<dyn Error>> {
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)
            .map_err(|error| format!("cannot open {}: {}", path.display(), error))?;
        // Two recordings into one file would each write the requests the other has written
        file.try_lock().map_err(|error| {
            format!(
                "{} is being recorded into by another replay: {}",
                path.display(),
                error
            )
        })?;
        let length = file.metadata()?.len();
        let (held, end) = if length == 0 {
            let mut line = serde_json::to_vec(&Header {
                format: FORMAT.to_owned(),
                version: VERSION,
            })?;
            line.push(b'\n');
            file.write_all(&line)?;
            (HashMap::new(), line.len() as u64)
        } else {
            let loaded = load(&file, path)?;
            if loaded.torn {
                tracing::warn!(
                    "cutting {} bytes of an unfinished line off the end of {}",
                    length - loaded.end,
                    path.display()
                );
                file.set_len(loaded.end)?;
            }
            (loaded.held, loaded.end)
        };

        let shared = Self::shared(path, file, held, end);
        let client = jsonrpc::Client::with_transport(Recording {
            core,
            shared: Arc::clone(&shared),
            wait,
        });
        Ok((CoreAnswers { shared }, client))
    }

    fn serving(path: &Path) -> Result<(Self, jsonrpc::Client), Box<dyn Error>> {
        let file = File::open(path)
            .map_err(|error| format!("cannot open {}: {}", path.display(), error))?;
        let loaded = load(&file, path)?;
        if loaded.torn {
            return Err(format!(
                "{} ends in an unfinished line; a recording that was stopped has to go on before its file is served",
                path.display()
            )
            .into());
        }

        let shared = Self::shared(path, file, loaded.held, loaded.end);
        let client = jsonrpc::Client::with_transport(Serving {
            shared: Arc::clone(&shared),
        });
        Ok((CoreAnswers { shared }, client))
    }

    fn shared(path: &Path, file: File, held: HashMap<Digest, Held>, end: u64) -> Arc<Shared> {
        Arc::new(Shared {
            path: path.to_path_buf(),
            state: Mutex::new(State {
                file,
                end,
                held,
                asked: BTreeMap::new(),
                added: 0,
                failure: None,
                finalizing: None,
            }),
        })
    }

    /// Why this run's answers can no longer be trusted, if they cannot: a request the file
    /// has no answer to, a request Core gave a recording no answer to, an answer that
    /// differs from the recorded one, or a file that cannot be read or written. Block
    /// execution may read such an error as Core being away for a moment, so the replay asks
    /// here after every request and stops.
    pub(super) fn failure(&self) -> Option<String> {
        self.shared.lock().failure.clone()
    }

    /// Executes `FinalizeBlock` through `finalize` and ends it at the first failure of
    /// Core's answers, before the block's transaction is committed. The error is that
    /// failure.
    ///
    /// Block execution reads a failed request to Core as Core being away for a moment, and
    /// where it hands Core the block's withdrawal transactions it notes the failure and goes
    /// on to the commit. The replay looks at its answers when a request has ended. It would
    /// find the failure with the block committed and no line of it written: its report
    /// could not be gone on with, and without a report a second start would pass the block
    /// over. So while `finalize` runs, a failure found on this thread unwinds from the
    /// request to Core it was found at, through the handler, to here. A recording has asked
    /// Core as often as it does by then.
    ///
    /// The handler is not changed for this and is left in the middle of a block. After an
    /// error the replay has to end, and execute nothing more on its platform. What still
    /// runs is the end of the platform itself, which flushes and closes GroveDB: that writes
    /// out what was committed before, and the block's transaction is dropped as it is. Any
    /// other unwinding, a panic of the node's code, goes on from here untouched.
    ///
    /// What this keeps is that the block's transaction is not committed. For the replay to
    /// be started again on the same database, more is needed. The database has to hold a
    /// block that was committed before: before its first block it holds no platform state,
    /// and a directory that was opened once is not taken for a new database again. And the
    /// block must have written nothing outside its transaction: on mainnet below height
    /// 32329 the node removes votes outside it, as the chain did then.
    pub(super) fn finalizing<T>(&self, finalize: impl FnOnce() -> T) -> Result<T, String> {
        self.shared.lock().finalizing = Some(thread::current().id());
        let outcome = panic::catch_unwind(AssertUnwindSafe(finalize));
        self.shared.lock().finalizing = None;
        match outcome {
            Ok(value) => Ok(value),
            Err(payload) if payload.is::<EndedBeforeCommit>() => Err(self
                .failure()
                .unwrap_or_else(|| "a request to Core failed".to_owned())),
            Err(payload) => panic::resume_unwind(payload),
        }
    }

    pub(super) fn summary(&self) -> CoreAnswersSummary {
        let state = self.shared.lock();
        CoreAnswersSummary {
            asked: state.asked.clone(),
            held: state.held.len() as u64,
            added: state.added,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::CoreRPCLike;
    use dpp::dashcore::BlockHash;
    use std::any::Any;
    use std::collections::VecDeque;
    use std::fs;
    use std::str::FromStr;

    const HASH_100: &str = "000000000000001bb2f3b1d7b9e1f3c5d7a9b1c3e5f7a9b1c3d5e7f9a1b3c5d7";
    const HASH_101: &str = "00000000000000aa11223344556677889900aabbccddeeff0011223344556677";
    const FORMAT_LINE: &str = "{\"format\":\"drive-abci core answers\",\"version\":1}\n";

    /// A Core node that answers each request with the next of the answers it was given.
    struct ScriptedCore {
        answers: Mutex<VecDeque<Result<Response, jsonrpc::Error>>>,
        asked: Arc<Mutex<Vec<String>>>,
    }

    type Asked = Arc<Mutex<Vec<String>>>;

    fn scripted_core(answers: Vec<Result<Response, jsonrpc::Error>>) -> (ScriptedCore, Asked) {
        let asked = Asked::default();
        (
            ScriptedCore {
                answers: Mutex::new(answers.into()),
                asked: Arc::clone(&asked),
            },
            asked,
        )
    }

    impl Transport for ScriptedCore {
        fn send_request(&self, request: Request) -> Result<Response, jsonrpc::Error> {
            self.asked.lock().unwrap().push(format!(
                "{} {}",
                request.method,
                params_text(request.params)
            ));
            self.answers
                .lock()
                .unwrap()
                .pop_front()
                .expect("the test's Core was asked more often than it has answers")
        }

        fn send_batch(&self, _: &[Request]) -> Result<Vec<Response>, jsonrpc::Error> {
            unreachable!("the node sends no batch")
        }

        fn fmt_target(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a scripted Core")
        }
    }

    fn result(json: &str) -> Result<Response, jsonrpc::Error> {
        Ok(Response {
            result: Some(RawValue::from_string(json.to_owned()).unwrap()),
            error: None,
            id: serde_json::Value::from(1),
            jsonrpc: None,
        })
    }

    fn hash(hex: &str) -> Result<Response, jsonrpc::Error> {
        result(&format!("\"{hex}\""))
    }

    fn rpc_error(code: i32, message: &str) -> Result<Response, jsonrpc::Error> {
        Ok(Response {
            result: None,
            error: Some(RpcError {
                code,
                message: message.to_owned(),
                data: None,
            }),
            id: serde_json::Value::from(1),
            jsonrpc: None,
        })
    }

    /// `getblockhash` of a height, asked of the client with no retry in between.
    fn ask(client: &jsonrpc::Client, height: u32) -> Result<String, String> {
        let params = RawValue::from_string(format!("[{height}]")).unwrap();
        let request = client.build_request("getblockhash", Some(&params));
        client
            .send_request(request)
            .and_then(|response| response.result::<String>())
            .map_err(|error| error.to_string())
    }

    fn recording(
        path: &Path,
        answers: Vec<Result<Response, jsonrpc::Error>>,
    ) -> (CoreAnswers, jsonrpc::Client, Asked) {
        let (core, asked) = scripted_core(answers);
        let (answers, client) = CoreAnswers::recording(path, core, Duration::ZERO).unwrap();
        (answers, client, asked)
    }

    fn refused(open: Result<(CoreAnswers, jsonrpc::Client), Box<dyn Error>>) -> String {
        match open {
            Ok(_) => panic!("the file was taken"),
            Err(error) => error.to_string(),
        }
    }

    fn answers_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("core-answers.jsonl")
    }

    #[test]
    fn should_serve_the_node_client_what_a_recording_kept_without_a_core_node() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (core, asked) = scripted_core(vec![hash(HASH_100), hash(HASH_101)]);
        let (answers, client) = CoreAnswers::recording(&path, core, Duration::ZERO).unwrap();
        let rpc = DefaultCoreRPC::with_client(Client::from_jsonrpc(client));
        assert_eq!(
            rpc.get_block_hash(100).unwrap(),
            BlockHash::from_str(HASH_100).unwrap()
        );
        assert_eq!(
            rpc.get_block_hash(101).unwrap(),
            BlockHash::from_str(HASH_101).unwrap()
        );
        assert_eq!(
            *asked.lock().unwrap(),
            vec!["getblockhash [100]", "getblockhash [101]"]
        );
        assert_eq!(
            answers.summary(),
            CoreAnswersSummary {
                asked: BTreeMap::from([("getblockhash".to_owned(), 2)]),
                held: 2,
                added: 2,
            }
        );
        assert_eq!(answers.failure(), None);
        drop(rpc);

        let (answers, rpc) = CoreAnswers::serve(&path).unwrap();
        // Asked in another order than recorded, and one of them twice
        assert_eq!(
            rpc.get_block_hash(101).unwrap(),
            BlockHash::from_str(HASH_101).unwrap()
        );
        assert_eq!(
            rpc.get_block_hash(100).unwrap(),
            BlockHash::from_str(HASH_100).unwrap()
        );
        assert_eq!(
            rpc.get_block_hash(101).unwrap(),
            BlockHash::from_str(HASH_101).unwrap()
        );
        assert_eq!(
            answers.summary(),
            CoreAnswersSummary {
                asked: BTreeMap::from([("getblockhash".to_owned(), 3)]),
                held: 2,
                added: 0,
            }
        );
        assert_eq!(answers.failure(), None);
    }

    #[test]
    fn should_write_the_format_line_and_one_line_for_each_request() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (answers, client, asked) = recording(&path, vec![hash(HASH_100), hash(HASH_100)]);
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        // The same request with the same answer is asked of Core and not written twice
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        assert_eq!(asked.lock().unwrap().len(), 2);
        assert_eq!(answers.summary().added, 1);

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!(
                "{FORMAT_LINE}{{\"method\":\"getblockhash\",\"params\":[100],\"result\":\"{HASH_100}\"}}\n"
            )
        );
    }

    #[test]
    fn should_keep_an_error_of_core_as_the_answer_and_serve_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (answers, client, _) =
            recording(&path, vec![rpc_error(-8, "Block height out of range")]);
        let recorded = ask(&client, 4_000_000).unwrap_err();
        assert!(recorded.contains("Block height out of range"), "{recorded}");
        assert_eq!(answers.failure(), None);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!(
                "{FORMAT_LINE}{{\"method\":\"getblockhash\",\"params\":[4000000],\"error\":{{\"code\":-8,\"message\":\"Block height out of range\",\"data\":null}}}}\n"
            )
        );

        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        assert_eq!(ask(&client, 4_000_000).unwrap_err(), recorded);
        assert_eq!(answers.failure(), None);
    }

    #[test]
    fn should_not_keep_an_error_that_says_core_is_not_ready() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (answers, client, asked) = recording(
            &path,
            vec![
                rpc_error(CORE_RPC_ERROR_IN_WARMUP, "Loading block index"),
                rpc_error(CORE_RPC_CLIENT_NOT_CONNECTED, "Dash Core is not connected"),
                rpc_error(
                    CORE_RPC_CLIENT_IN_INITIAL_DOWNLOAD,
                    "Dash Core is downloading",
                ),
                hash(HASH_100),
            ],
        );
        // The recording asks again after each of them, and gives the answer
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        assert_eq!(asked.lock().unwrap().len(), 4);
        assert_eq!(answers.summary().held, 1);
        assert_eq!(answers.failure(), None);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!(
                "{FORMAT_LINE}{{\"method\":\"getblockhash\",\"params\":[100],\"result\":\"{HASH_100}\"}}\n"
            )
        );
    }

    #[test]
    fn should_refuse_a_second_different_answer_while_recording() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (answers, client, asked) = recording(&path, vec![hash(HASH_100), hash(HASH_101)]);
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);

        let expected = format!(
            "Core answered `getblockhash` [100] differently from the answer {} holds for it; a recording needs a Core whose chain stands still",
            path.display()
        );
        let error = ask(&client, 100).unwrap_err();
        assert!(error.contains(&expected), "{error}");
        assert_eq!(answers.failure(), Some(expected.clone()));

        // After that nothing more is asked of Core: every request is refused alike
        let error = ask(&client, 101).unwrap_err();
        assert!(error.contains(&expected), "{error}");
        assert_eq!(asked.lock().unwrap().len(), 2);

        // The file keeps the first answer only
        let (_, client) = CoreAnswers::serving(&path).unwrap();
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
    }

    #[test]
    fn should_refuse_a_request_the_file_has_no_answer_to() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();

        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        let expected = format!(
            "{} holds no answer of Core to `getblockhash` [101]",
            path.display()
        );
        let error = ask(&client, 101).unwrap_err();
        assert!(error.contains(&expected), "{error}");
        assert_eq!(answers.failure(), Some(expected.clone()));

        // A request it does hold is refused from then on too: the run has failed
        let error = ask(&client, 100).unwrap_err();
        assert!(error.contains(&expected), "{error}");
    }

    #[test]
    fn should_go_on_recording_into_a_file_that_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();
        drop(client);

        // The second run asks the held request again and a new one
        let (answers, client, _) = recording(&path, vec![hash(HASH_100), hash(HASH_101)]);
        ask(&client, 100).unwrap();
        ask(&client, 101).unwrap();
        assert_eq!(
            answers.summary(),
            CoreAnswersSummary {
                asked: BTreeMap::from([("getblockhash".to_owned(), 2)]),
                held: 2,
                added: 1,
            }
        );
        drop(client);

        assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 3);
        let (_, client) = CoreAnswers::serving(&path).unwrap();
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        assert_eq!(ask(&client, 101).unwrap(), HASH_101);
    }

    #[test]
    fn should_refuse_a_second_recording_into_a_file_while_one_runs() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        let (core, _) = scripted_core(vec![]);
        let error = refused(CoreAnswers::recording(&path, core, Duration::ZERO));
        assert!(
            error.contains("is being recorded into by another replay"),
            "{error}"
        );

        // The first goes on, and when it has ended the file can be recorded into again
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        drop(client);
        let (answers, _client, _) = recording(&path, vec![]);
        assert_eq!(answers.summary().held, 1);
    }

    #[test]
    fn should_refuse_a_different_answer_to_a_request_an_earlier_run_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();
        drop(client);

        let (answers, client, _) = recording(&path, vec![hash(HASH_101)]);
        let error = ask(&client, 100).unwrap_err();
        assert!(error.contains("differently"), "{error}");
        assert!(answers.failure().is_some());
        assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 2);
    }

    #[test]
    fn should_cut_an_unfinished_line_off_when_recording_goes_on_and_refuse_to_serve_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();
        drop(client);
        let whole = fs::read_to_string(&path).unwrap();
        fs::write(&path, format!("{whole}{{\"method\":\"getblockhash\",\"par")).unwrap();

        let error = refused(CoreAnswers::serving(&path));
        assert!(error.contains("ends in an unfinished line"), "{error}");

        let (_, client, _) = recording(&path, vec![hash(HASH_101)]);
        ask(&client, 101).unwrap();
        drop(client);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!(
                "{whole}{{\"method\":\"getblockhash\",\"params\":[101],\"result\":\"{HASH_101}\"}}\n"
            )
        );
    }

    #[test]
    fn should_refuse_a_file_that_is_not_one_of_core_answers() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let error = refused(CoreAnswers::serving(&path));
        assert!(error.contains("cannot open"), "{error}");

        fs::write(&path, "").unwrap();
        let error = refused(CoreAnswers::serving(&path));
        assert!(error.contains("has no format line"), "{error}");

        fs::write(&path, "{\"timestamp\":\"2026-01-01\"}\n").unwrap();
        let error = refused(CoreAnswers::serving(&path));
        assert!(
            error.contains("does not start with the line of a file of Core's answers"),
            "{error}"
        );

        fs::write(
            &path,
            "{\"format\":\"drive-abci core answers\",\"version\":2}\n",
        )
        .unwrap();
        let error = refused(CoreAnswers::serving(&path));
        assert!(
            error.contains(
                "is `drive-abci core answers` version 2; this replay reads `drive-abci core answers` version 1"
            ),
            "{error}"
        );

        fs::write(
            &path,
            format!("{FORMAT_LINE}{{\"method\":\"getblockhash\"}}\n"),
        )
        .unwrap();
        let error = refused(CoreAnswers::serving(&path));
        assert!(
            error.contains("line 2 of") && error.contains("is not a request and its answer"),
            "{error}"
        );

        let line =
            format!("{{\"method\":\"getblockhash\",\"params\":[100],\"result\":\"{HASH_100}\"}}\n");
        fs::write(&path, format!("{FORMAT_LINE}{line}{line}")).unwrap();
        let error = refused(CoreAnswers::serving(&path));
        assert!(
            error.contains("line 3 of")
                && error
                    .contains("answers `getblockhash` [100], which an earlier line answers too"),
            "{error}"
        );

        // A recording does not go on in such a file either
        let (core, _) = scripted_core(vec![]);
        let error = refused(CoreAnswers::recording(&path, core, Duration::ZERO));
        assert!(error.contains("line 3 of"), "{error}");

        fs::write(&path, "{\"format\":\"drive-abci core").unwrap();
        let (core, _) = scripted_core(vec![]);
        let error = refused(CoreAnswers::recording(&path, core, Duration::ZERO));
        assert!(error.contains("has no format line"), "{error}");
    }

    #[test]
    fn should_write_an_answer_that_spans_lines_on_one_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![result(&format!("\n  \"{HASH_100}\"\r\n"))]);
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);

        assert_eq!(fs::read_to_string(&path).unwrap().lines().count(), 2);
        let (_, client) = CoreAnswers::serving(&path).unwrap();
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
    }

    #[test]
    fn should_ask_again_after_an_error_of_the_connection_and_keep_nothing_of_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (answers, client, asked) = recording(&path, vec![away(), hash(HASH_100)]);
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        assert_eq!(asked.lock().unwrap().len(), 2);
        assert_eq!(answers.failure(), None);
        assert_eq!(answers.summary().held, 1);
        assert_eq!(answers.summary().added, 1);
    }

    #[test]
    fn should_end_a_recording_at_an_error_the_node_client_does_not_ask_again_after() {
        // A reply that is no JSON, which the node's client hands to block execution as the
        // error it is, and another error that is none of the connection
        let no_json = || {
            jsonrpc::Error::Json(serde_json::from_str::<serde_json::Value>("<html>").unwrap_err())
        };
        for error in [no_json(), jsonrpc::Error::EmptyBatch] {
            let dir = tempfile::tempdir().unwrap();
            let path = answers_path(&dir);
            let said = error.to_string();
            // Core would have answered a second try
            let (answers, client, asked) = recording(&path, vec![Err(error), hash(HASH_100)]);
            let expected = format!(
                "Core's reply to `getblockhash` [100] is no answer: {}; the node's client does not ask again after such an error, and the recording ends here, since {} holds answers of Core and nothing else",
                said,
                path.display()
            );
            let noted = ask(&client, 100).unwrap_err();
            assert!(noted.contains(&expected), "{noted}");
            // It was not asked again: the answer to a second try is one the node would not
            // have been given
            assert_eq!(asked.lock().unwrap().len(), 1);
            assert_eq!(answers.failure(), Some(expected));
            assert_eq!(fs::read_to_string(&path).unwrap(), FORMAT_LINE);
        }
    }

    #[test]
    fn should_fail_the_run_when_an_answer_cannot_be_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();
        drop(client);

        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        // The file is cut short under the replay that serves it
        fs::write(&path, FORMAT_LINE).unwrap();
        let error = ask(&client, 100).unwrap_err();
        assert!(
            error.contains("the answer of Core to `getblockhash` [100] cannot be read from"),
            "{error}"
        );
        assert!(answers.failure().is_some());
    }

    #[test]
    fn should_stop_a_recording_where_core_refuses_a_method_by_http_status() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let forbidden = || {
            Err(jsonrpc::Error::Transport(Box::new(
                HttpError::HttpErrorCode(403),
            )))
        };
        let (answers, client, asked) = recording(&path, vec![forbidden(), hash(HASH_100)]);
        let expected = format!(
            "Core refused `getblockhash` with HTTP status 403; a recording needs every method block execution asks for allowed to its RPC user, since {} holds answers and no refusals",
            path.display()
        );
        let error = ask(&client, 100).unwrap_err();
        assert!(error.contains(&expected), "{error}");
        assert_eq!(answers.failure(), Some(expected));
        // Nothing more is asked of Core
        ask(&client, 100).unwrap_err();
        assert_eq!(asked.lock().unwrap().len(), 1);

        // Another HTTP status is an error of the connection, after which it asks again
        let other_dir = tempfile::tempdir().unwrap();
        let (answers, client, asked) =
            recording(&answers_path(&other_dir), vec![away(), hash(HASH_100)]);
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        assert_eq!(asked.lock().unwrap().len(), 2);
        assert_eq!(answers.failure(), None);
    }

    #[test]
    fn should_refuse_an_https_address_as_the_node_client_does() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let error = match CoreAnswers::record(
            &path,
            "https://127.0.0.1:9998",
            "user".to_owned(),
            "password".to_owned(),
        ) {
            Ok(_) => panic!("a recording was opened over an https address"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("https is not supported"), "{error}");
        assert!(!path.exists());

        // The node's client is what a recording asks through
        let (_, rpc) = CoreAnswers::record(
            &path,
            "http://127.0.0.1:9998",
            "user".to_owned(),
            "password".to_owned(),
        )
        .unwrap();
        let debug = format!("{rpc:?}");
        assert!(debug.contains("recorded in"), "{debug}");
        let node = NodeClient(
            Client::new(
                "http://127.0.0.1:9998",
                Auth::UserPass("user".to_owned(), "password".to_owned()),
            )
            .unwrap(),
        );
        let client = jsonrpc::Client::with_transport(node);
        let request = client.build_request("getblockhash", None);
        let error = client.send_batch(&[request]).unwrap_err().to_string();
        assert!(
            error.contains("a batch of requests to Core is not recorded"),
            "{error}"
        );
    }

    #[test]
    fn should_write_the_data_of_an_error_on_one_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(
            &path,
            vec![Ok(Response {
                result: None,
                error: Some(RpcError {
                    code: -26,
                    message: "bad-txns".to_owned(),
                    data: Some(
                        RawValue::from_string(
                            "{\n  \"reject\": [1,\n 2.50],\r\n \"text\": \"a \\\"b\\n c\\\\\",\n \"reject\": 0.10000000000000000001\n}"
                                .to_owned(),
                        )
                        .unwrap(),
                    ),
                }),
                id: serde_json::Value::from(1),
                jsonrpc: None,
            })],
        );
        let recorded = ask(&client, 100).unwrap_err();
        drop(client);

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!(
                "{FORMAT_LINE}{{\"method\":\"getblockhash\",\"params\":[100],\"error\":{{\"code\":-26,\"message\":\"bad-txns\",\"data\":{{  \"reject\": [1, 2.50], \"text\": \"a \\\"b\\n c\\\\\", \"reject\": 0.10000000000000000001}}}}}}\n"
            )
        );
        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        let served = ask(&client, 100).unwrap_err();
        assert!(served.contains("bad-txns"), "{served}");
        assert!(recorded.contains("bad-txns"), "{recorded}");
        assert_eq!(answers.failure(), None);
    }

    #[test]
    fn should_cut_off_a_line_that_ends_inside_a_character() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();
        drop(client);
        let whole = fs::read(&path).unwrap();
        // The first of the two bytes of a character of an error's message
        let mut torn = whole.clone();
        torn.extend_from_slice(b"{\"method\":\"getblockhash\",\"params\":[101],\"error\":{\"code\":-1,\"message\":\"\xc3");
        fs::write(&path, &torn).unwrap();

        let error = refused(CoreAnswers::serving(&path));
        assert!(error.contains("ends in an unfinished line"), "{error}");

        let (answers, client, _) = recording(&path, vec![hash(HASH_101)]);
        assert_eq!(answers.summary().held, 1);
        assert_eq!(fs::read(&path).unwrap(), whole);
        ask(&client, 101).unwrap();
        drop(client);
        let (_, client) = CoreAnswers::serving(&path).unwrap();
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        assert_eq!(ask(&client, 101).unwrap(), HASH_101);

        // Bytes that are no text inside a whole line are not a torn end
        let mut bad = whole.clone();
        bad.extend_from_slice(b"\xff\xfe\n");
        fs::write(&path, &bad).unwrap();
        let error = refused(CoreAnswers::serving(&path));
        assert!(
            error.contains("line 3 of") && error.contains("is not text"),
            "{error}"
        );
    }

    #[test]
    fn should_not_serve_a_line_that_was_changed_after_the_file_was_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![hash(HASH_100), hash(HASH_101)]);
        ask(&client, 100).unwrap();
        ask(&client, 101).unwrap();
        drop(client);
        let whole = fs::read_to_string(&path).unwrap();

        // Another answer of the same length is written over the answer to the first request
        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        let other_answer = whole.replacen(HASH_100, HASH_101, 1);
        assert_eq!(other_answer.len(), whole.len());
        fs::write(&path, &other_answer).unwrap();
        let error = ask(&client, 100).unwrap_err();
        assert!(
            error.contains("the file was changed after the replay had read it"),
            "{error}"
        );
        assert!(answers.failure().is_some());

        // The line of another request is put where this request's line was
        fs::write(&path, &whole).unwrap();
        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        fs::write(&path, whole.replacen("[100]", "[102]", 1)).unwrap();
        let error = ask(&client, 100).unwrap_err();
        assert!(
            error.contains("the file was changed after the replay had read it"),
            "{error}"
        );
        assert!(answers.failure().is_some());
    }

    #[test]
    fn should_refuse_batches_and_name_what_it_asks() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(&path, vec![]);
        let request = client.build_request("getblockhash", None);
        let error = client.send_batch(&[request]).unwrap_err().to_string();
        assert!(
            error.contains("a batch of requests to Core is not recorded"),
            "{error}"
        );
        assert_eq!(
            format!("{client:?}"),
            format!(
                "jsonrpc::Client(a scripted Core, recorded in {})",
                path.display()
            )
        );
        drop(client);

        let (_, client) = CoreAnswers::serving(&path).unwrap();
        let request = client.build_request("getblockhash", None);
        let error = client.send_batch(&[request]).unwrap_err().to_string();
        assert!(
            error.contains("a batch of requests to Core is not answered from a file"),
            "{error}"
        );
        assert_eq!(
            format!("{client:?}"),
            format!("jsonrpc::Client(Core's answers in {})", path.display())
        );
    }

    #[test]
    fn should_tell_requests_apart_by_method_and_parameters() {
        assert_ne!(
            request_digest("getblockhash", "[1]"),
            request_digest("getblockhash", "[2]")
        );
        assert_ne!(
            request_digest("getblock", "[1]"),
            request_digest("getblockhash", "[1]")
        );
        // The method ends where its name ends
        assert_ne!(request_digest("a", "b[1]"), request_digest("ab", "[1]"));
        assert_eq!(params_text(None), "null");
    }

    #[test]
    fn should_answer_a_request_without_parameters_and_a_result_that_is_null() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        let (_, client, _) = recording(
            &path,
            vec![Ok(Response {
                result: None,
                error: None,
                id: serde_json::Value::from(1),
                jsonrpc: None,
            })],
        );
        let request = client.build_request("ping", None);
        let response = client.send_request(request).unwrap();
        assert!(response.is_none());
        drop(client);

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!("{FORMAT_LINE}{{\"method\":\"ping\",\"params\":null}}\n")
        );
        let (_, client) = CoreAnswers::serving(&path).unwrap();
        let request = client.build_request("ping", None);
        let response = client.send_request(request).unwrap();
        assert!(response.is_none());
        assert!(response.error.is_none());
        assert_eq!(response.jsonrpc.as_deref(), Some("2.0"));
    }

    // ---- a request Core does not answer ----

    /// An error of the connection: Core did not answer.
    fn away() -> Result<Response, jsonrpc::Error> {
        Err(jsonrpc::Error::Transport(Box::new(
            HttpError::HttpErrorCode(503),
        )))
    }

    #[test]
    fn should_keep_the_answer_core_gives_to_the_last_try() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        // Core is away three times and answers the fourth, the last time it is asked. The
        // node's client is given the answer, and has seen nothing of the three
        let (core, asked) = scripted_core(vec![away(), away(), away(), hash(HASH_100)]);
        let (answers, client) = CoreAnswers::recording(&path, core, Duration::ZERO).unwrap();
        let rpc = DefaultCoreRPC::with_client(Client::from_jsonrpc(client));
        assert_eq!(
            rpc.get_block_hash(100).unwrap(),
            BlockHash::from_str(HASH_100).unwrap()
        );
        assert_eq!(asked.lock().unwrap().len(), 4);
        assert_eq!(answers.failure(), None);
        assert_eq!(
            answers.summary(),
            CoreAnswersSummary {
                asked: BTreeMap::from([("getblockhash".to_owned(), 1)]),
                held: 1,
                added: 1,
            }
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!(
                "{FORMAT_LINE}{{\"method\":\"getblockhash\",\"params\":[100],\"result\":\"{HASH_100}\"}}\n"
            )
        );
        drop(rpc);

        let (_, rpc) = CoreAnswers::serve(&path).unwrap();
        assert_eq!(
            rpc.get_block_hash(100).unwrap(),
            BlockHash::from_str(HASH_100).unwrap()
        );
    }

    #[test]
    fn should_fail_a_recording_where_core_gives_no_answer_to_any_try() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        // Core is away all four times, and would have answered a fifth
        let (answers, client, asked) =
            recording(&path, vec![away(), away(), away(), away(), hash(HASH_100)]);
        let expected = format!(
            "Core gave no answer to `getblockhash` [100] in 4 tries, the last time: {}; the recording ends here, since {} holds answers of Core and nothing else",
            away().unwrap_err(),
            path.display()
        );
        // What asked goes on without the answer, as the broadcast of a withdrawal does
        let noted = ask(&client, 100).unwrap_err();
        assert!(noted.contains(&expected), "{noted}");
        assert_eq!(asked.lock().unwrap().len(), 4);
        assert_eq!(answers.failure(), Some(expected.clone()));
        assert_eq!(fs::read_to_string(&path).unwrap(), FORMAT_LINE);

        // The run has failed. A second call with the same request, which Core would answer
        // now, is not given an answer the first was not given, and no other request is
        let again = ask(&client, 100).unwrap_err();
        assert!(again.contains(&expected), "{again}");
        ask(&client, 101).unwrap_err();
        assert_eq!(asked.lock().unwrap().len(), 4);
        assert_eq!(fs::read_to_string(&path).unwrap(), FORMAT_LINE);

        // Core's error for a node that is not ready is no answer either
        let other_dir = tempfile::tempdir().unwrap();
        let warming_up = || rpc_error(CORE_RPC_ERROR_IN_WARMUP, "Loading block index");
        let (answers, client, _) = recording(
            &answers_path(&other_dir),
            vec![warming_up(), away(), warming_up(), warming_up()],
        );
        ask(&client, 100).unwrap_err();
        let failure = answers.failure().unwrap();
        assert!(
            failure.starts_with(&format!(
                "Core gave no answer to `getblockhash` [100] in 4 tries, the last time: its error {} (Loading block index); ",
                CORE_RPC_ERROR_IN_WARMUP
            )),
            "{failure}"
        );
    }

    #[test]
    fn should_wait_longer_each_time_before_it_asks_core_again() {
        let dir = tempfile::tempdir().unwrap();
        let (core, _) = scripted_core(vec![away(), away(), away(), hash(HASH_100)]);
        let wait = Duration::from_millis(20);
        let (_, client) = CoreAnswers::recording(&answers_path(&dir), core, wait).unwrap();
        let start = std::time::Instant::now();
        assert_eq!(ask(&client, 100).unwrap(), HASH_100);
        // Once, twice and three times the wait
        assert!(start.elapsed() >= wait * 6, "{:?}", start.elapsed());
    }

    // ---- FinalizeBlock and a failure of the answers ----

    /// What `FinalizeBlock` does where it hands Core a withdrawal transaction: it asks,
    /// notes a failure, and goes on to commit the block.
    fn finalize(client: &jsonrpc::Client, height: u32, committed: &mut bool) {
        let _noted = ask(client, height);
        *committed = true;
    }

    #[test]
    fn should_end_a_finalization_before_its_commit_where_the_file_has_no_answer() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);
        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();
        drop(client);

        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        let mut committed = false;
        let failure = answers
            .finalizing(|| finalize(&client, 101, &mut committed))
            .unwrap_err();
        assert_eq!(
            failure,
            format!(
                "{} holds no answer of Core to `getblockhash` [101]",
                path.display()
            )
        );
        assert!(!committed);
        assert_eq!(answers.failure(), Some(failure));
        drop(client);

        // The file is given the answer: the same finalization goes through to its commit
        let (_, client, _) = recording(&path, vec![hash(HASH_101)]);
        ask(&client, 101).unwrap();
        drop(client);
        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        let mut committed = false;
        answers
            .finalizing(|| finalize(&client, 101, &mut committed))
            .unwrap();
        assert!(committed);
        assert_eq!(answers.failure(), None);

        // Outside a finalization a failure is the request's error, which the replay looks
        // for when the request has ended
        let error = ask(&client, 102).unwrap_err();
        assert!(error.contains("holds no answer of Core to"), "{error}");
        assert!(answers.failure().is_some());
    }

    #[test]
    fn should_end_a_finalization_of_a_recording_where_core_gives_no_answer() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);

        // Core is away once and then answers: the finalization is given the answer and
        // goes through to its commit
        let (answers, client, asked) = recording(&path, vec![away(), hash(HASH_100)]);
        let mut committed = false;
        answers
            .finalizing(|| finalize(&client, 100, &mut committed))
            .unwrap();
        assert!(committed);
        assert_eq!(asked.lock().unwrap().len(), 2);
        assert_eq!(answers.failure(), None);
        // The file is the recording's while the recording is there
        drop((answers, client));

        // Core gives no answer to any try: the finalization ends there, not committed
        let (answers, client, asked) = recording(&path, vec![away(), away(), away(), away()]);
        let mut committed = false;
        let failure = answers
            .finalizing(|| finalize(&client, 101, &mut committed))
            .unwrap_err();
        assert!(
            failure.starts_with("Core gave no answer to `getblockhash` [101] in 4 tries, "),
            "{failure}"
        );
        assert!(!committed);
        assert_eq!(asked.lock().unwrap().len(), 4);
        drop((answers, client));

        // An answer that differs from the recorded one ends it the same way
        let (answers, client, _) = recording(&path, vec![hash(HASH_101)]);
        let mut committed = false;
        let failure = answers
            .finalizing(|| finalize(&client, 100, &mut committed))
            .unwrap_err();
        assert!(failure.contains("differently from the answer"), "{failure}");
        assert!(!committed);
    }

    #[test]
    fn should_take_only_its_own_unwinding_for_a_failure_of_the_answers() {
        let dir = tempfile::tempdir().unwrap();
        let path = answers_path(&dir);
        let (_, client, _) = recording(&path, vec![hash(HASH_100)]);
        ask(&client, 100).unwrap();
        drop(client);
        let (answers, client) = CoreAnswers::serving(&path).unwrap();

        // A panic of what is executed is not the replay's to take
        let payload: Box<dyn Any + Send> = panic::catch_unwind(AssertUnwindSafe(|| {
            let _ended: Result<(), String> =
                answers.finalizing(|| panic!("of the node's own code"));
        }))
        .unwrap_err();
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"of the node's own code")
        );
        assert_eq!(answers.failure(), None);

        // A failure found on another thread is that thread's error, and unwinds nothing
        let mut committed = false;
        answers
            .finalizing(|| {
                let asked = thread::scope(|scope| scope.spawn(|| ask(&client, 101)).join());
                assert!(asked.expect("the thread did not unwind").is_err());
                committed = true;
            })
            .unwrap();
        assert!(committed);
        assert!(answers.failure().is_some());

        // After a finalization a failure is an error again
        let (answers, client) = CoreAnswers::serving(&path).unwrap();
        answers.finalizing(|| ()).unwrap();
        ask(&client, 101).unwrap_err();
        assert!(answers.failure().is_some());
    }
}
