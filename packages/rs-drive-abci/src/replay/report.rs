//! What a replay holds every block against: the logged answers of the node that wrote the log,
//! and another run's report.
//!
//! A replay can write a report, one JSON line for each block it finalized, of what the block
//! left: the app hash, GroveDB's root hash, every state transition's result code and
//! `gas_used`, and a hash over each whole answer. A line has nothing of the machine or the
//! time in it, so two replays of the same log that answered every request alike and left the
//! same root hashes, on two storage engines for one, write the same bytes:
//!
//! ```text
//! {"height":4,"round":0,"core_height":2128900,"protocol_version":1,"app_hash":"26a5…","grove_root":"26a5…","root_is_app_hash":true,"tx":[[0,117438280]],"held_against_log":true,"proposal_sha256":"…","finalize_sha256":"…"}
//! ```
//!
//! `tx` has `[code, gas_used]` for each state transition of the block, in the block's order.
//! `gas_used` is the fee the result of a state transition states: its storage fee plus its
//! processing fee, before refunds. `held_against_log` says whether that answer was held against
//! the answer the log has for the same request and found equal in status, app hash and every
//! `[code, gas_used]`. Which line of the log is taken for the answer to a request, and when
//! none is, is said at `log_ingest`'s `read_logged_answer`: in short, the first request or
//! answer line after the request's, if nothing unreadable lies before it and it is an answer
//! that names the thread the request's line names.
//!
//! `core_height`, `tx` and `proposal_sha256` come from the replay's answer to the proposal of
//! the block, and the replay takes an answer for the block's only in one case: the request it
//! executed directly before `FinalizeBlock` was a `ProcessProposal` of the same height, round
//! and block hash, and it accepted the proposal. Only `info` requests, which read the state
//! and can replace nothing of a block's execution, may lie between the two. In every other
//! case (another request in between, another round or block, a refused proposal) the replay
//! cannot tell that the answer it holds is of the execution that is finalized: the three
//! fields are then `null` and
//! `held_against_log` is `false`. So a report says that every block's fees were compared only
//! where no line has a `null`, and that they were held against the log only where every line
//! has `held_against_log` true; the run's last log lines count both.

use crate::replay::chain_answer::{ChainProposalAnswer, LoggedAnswer, TxOutcome};
use crate::replay::core_answers::CoreAnswers;
use dpp::dashcore::hashes::sha256;
use prost::Message;
use same_file::Handle;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::iter;
use std::path::{Path, PathBuf};
use std::str;
use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
use tenderdash_abci::proto::abci::{ResponseFinalizeBlock, ResponseProcessProposal};

/// One line of a report: what a block left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BlockRecord {
    pub(super) height: u64,
    pub(super) round: u32,
    /// The core chain locked height the block was proposed with
    pub(super) core_height: Option<u32>,
    /// The protocol version in consensus after the block
    pub(super) protocol_version: u32,
    /// The app hash of the platform state after the block, in hex
    pub(super) app_hash: String,
    /// GroveDB's root hash after the block, in hex
    pub(super) grove_root: String,
    pub(super) root_is_app_hash: bool,
    /// The result code and `gas_used` of each state transition, in the block's order
    pub(super) tx: Option<Vec<TxOutcome>>,
    /// Whether the answer `tx` is of was held against the log's answer and found equal
    pub(super) held_against_log: bool,
    /// SHA-256 of the whole `ProcessProposal` answer, encoded as it goes to Tenderdash
    pub(super) proposal_sha256: Option<String>,
    /// SHA-256 of the whole `FinalizeBlock` answer, encoded as it goes to Tenderdash
    pub(super) finalize_sha256: String,
}

fn sha256_hex(message: &impl Message) -> String {
    hex::encode(sha256::Hash::hash(&message.encode_to_vec()).to_byte_array())
}

/// The fields of a report line, in the order they are written.
const FIELDS: [&str; 11] = [
    "height",
    "round",
    "core_height",
    "protocol_version",
    "app_hash",
    "grove_root",
    "root_is_app_hash",
    "tx",
    "held_against_log",
    "proposal_sha256",
    "finalize_sha256",
];

/// A report line, if the text is one: an object of exactly the fields a line is written with,
/// those that may be `null` among them.
fn record_of(line: &str) -> Option<BlockRecord> {
    let Value::Object(fields) = serde_json::from_str(line).ok()? else {
        return None;
    };
    if !fields.keys().map(String::as_str).eq(FIELDS) {
        return None;
    }
    // From the text and not from the object: a field the text has twice is in the object once
    serde_json::from_str(line).ok()
}

/// A line of a report with its number: `None` for a line that is not a whole line of a report
/// (no line end, not text, not a report line).
type NumberedLine = (u64, Option<(String, BlockRecord)>);

/// The lines of a report read from `reader`.
fn records(mut reader: impl BufRead) -> impl Iterator<Item = io::Result<NumberedLine>> {
    let mut number = 0u64;
    let mut bytes = Vec::new();
    iter::from_fn(move || {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => None,
            Ok(_) => {
                number += 1;
                let whole = bytes
                    .strip_suffix(b"\n")
                    .and_then(|line| str::from_utf8(line).ok())
                    .and_then(|line| Some((line.to_owned(), record_of(line)?)));
                Some(Ok((number, whole)))
            }
            Err(error) => Some(Err(error)),
        }
    })
}

/// The block a replay's database is at when the replay starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DatabaseAt {
    pub(super) height: u64,
    /// The app hash of the platform state the database holds
    pub(super) app_hash: Vec<u8>,
}

/// Writes a report, a line a block.
#[derive(Debug)]
struct ReportWriter {
    path: PathBuf,
    file: File,
}

impl ReportWriter {
    /// Opens the report at `path` for a replay whose database is at `database`, `None` for a
    /// new database. The report of a new database has no lines yet. The report of a database
    /// that has blocks is added to, and has to be that database's: whole lines of a report,
    /// one block after the other, the last of them of the block the database is at and with
    /// its app hash. A report with a line missing or a line too many is not the database's
    /// history, and neither is a report with no lines: the line of a block that was found to
    /// differ is not written. At which block the report begins is not looked at here, since
    /// the database does not say which block was its first; a report that is held against
    /// another has to begin where that one does (`ReportReader::line_for`).
    fn open(path: &Path, database: Option<&DatabaseAt>) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)
            .map_err(|error| format!("cannot open report {}: {}", path.display(), error))?;

        let mut last: Option<BlockRecord> = None;
        for line in records(BufReader::new(&file)) {
            let (line_number, whole) =
                line.map_err(|error| format!("cannot read report {}: {}", path.display(), error))?;
            // A line a stopped replay did not finish has no line end
            let (_, record) = whole.ok_or_else(|| {
                format!(
                    "line {} of report {} is not a whole line of a report",
                    line_number,
                    path.display()
                )
            })?;
            let follows = last
                .as_ref()
                .is_none_or(|earlier| earlier.height.checked_add(1) == Some(record.height));
            if !follows {
                return Err(format!(
                    "line {} of report {} is for height {} and does not follow the line before it",
                    line_number,
                    path.display(),
                    record.height
                ));
            }
            last = Some(record);
        }

        match (last, database) {
            (None, None) => {}
            (None, Some(database)) => {
                return Err(format!(
                    "report {} has no lines, and the database is at height {}; a report is written from a new database on",
                    path.display(),
                    database.height
                ));
            }
            (Some(last), None) => {
                return Err(format!(
                    "report {} ends at height {}, and the database is new; a report goes on only from the block its database is at",
                    path.display(),
                    last.height
                ));
            }
            (Some(last), Some(database)) => {
                if last.height != database.height {
                    return Err(format!(
                        "report {} ends at height {}, and the database is at height {}; a report goes on only from the block its database is at",
                        path.display(),
                        last.height,
                        database.height
                    ));
                }
                if !last
                    .app_hash
                    .eq_ignore_ascii_case(&hex::encode(&database.app_hash))
                {
                    return Err(format!(
                        "report {} ends at height {} with app hash {}, and the database has app hash {} there; the report is of another database",
                        path.display(),
                        last.height,
                        last.app_hash,
                        hex::encode(&database.app_hash)
                    ));
                }
            }
        }

        Ok(ReportWriter {
            path: path.to_path_buf(),
            file,
        })
    }

    fn write(&mut self, line: &str) -> Result<(), String> {
        let mut bytes = Vec::with_capacity(line.len() + 1);
        bytes.extend_from_slice(line.as_bytes());
        bytes.push(b'\n');
        self.file
            .write_all(&bytes)
            .map_err(|error| format!("cannot write report {}: {}", self.path.display(), error))
    }
}

/// Reads another run's report in step with the replay: its lines one after the other, the
/// first of them for the first block this replay's report has.
struct ReportReader {
    path: PathBuf,
    lines: Box<dyn Iterator<Item = io::Result<NumberedLine>>>,
}

impl fmt::Debug for ReportReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReportReader({})", self.path.display())
    }
}

impl ReportReader {
    /// Opens the other report, and gives the identity of the file that was opened.
    fn open(path: &Path) -> Result<(Self, Handle), String> {
        let open_error =
            |error: io::Error| format!("cannot open report {}: {}", path.display(), error);
        let file = File::open(path).map_err(open_error)?;
        let identity = file
            .try_clone()
            .and_then(Handle::from_file)
            .map_err(open_error)?;
        Ok((
            ReportReader {
                path: path.to_path_buf(),
                lines: Box::new(records(BufReader::new(file))),
            },
            identity,
        ))
    }

    /// The other run's next line, which has to be for the block at `height`: the two reports
    /// start at the same block and neither leaves one out.
    fn line_for(&mut self, height: u64) -> Result<String, String> {
        let (line_number, whole) = match self.lines.next() {
            Some(line) => line.map_err(|error| {
                format!("cannot read report {}: {}", self.path.display(), error)
            })?,
            None => {
                return Err(format!(
                    "report {} has no line for height {}: it ends before",
                    self.path.display(),
                    height
                ))
            }
        };
        match whole {
            Some((line, record)) if record.height == height => Ok(line),
            Some((_, record)) => Err(format!(
                "report {} has no line for height {}: line {} is for height {}",
                self.path.display(),
                height,
                line_number,
                record.height
            )),
            None => Err(format!(
                "line {} of report {} is not a whole line of a report",
                line_number,
                self.path.display()
            )),
        }
    }

    /// Holds a line of this replay against the other run's line for the same block. The
    /// error names each field that differs and gives both lines.
    fn hold(&mut self, height: u64, line: &str) -> Result<(), String> {
        let other_line = self.line_for(height)?;
        if other_line == line {
            return Ok(());
        }
        let mut lines: Vec<String> = differences(line, &other_line)
            .iter()
            .map(|difference| format!("DIFFERS height={height} {difference}"))
            .collect();
        lines.push(format!("DIFFERS height={height} this={line}"));
        lines.push(format!("DIFFERS height={height} other={other_line}"));
        Err(lines.join("\n"))
    }
}

/// The fields two report lines differ in, each as `field=<name> this=<value> other=<value>`.
/// A list that differs is given by its places that differ, and by its length if that does.
fn differences(this: &str, other: &str) -> Vec<String> {
    let (Ok(Value::Object(this_fields)), Ok(Value::Object(other_fields))) = (
        serde_json::from_str::<Value>(this),
        serde_json::from_str::<Value>(other),
    ) else {
        return vec!["field=line this=<not a line of a report, or the other is not>".to_owned()];
    };

    let absent = Value::String("<absent>".to_owned());
    let mut names: Vec<&String> = this_fields.keys().collect();
    names.extend(
        other_fields
            .keys()
            .filter(|name| !this_fields.contains_key(*name)),
    );

    let mut found = Vec::new();
    for name in names {
        let this_value = this_fields.get(name).unwrap_or(&absent);
        let other_value = other_fields.get(name).unwrap_or(&absent);
        match (this_value, other_value) {
            (Value::Array(this_items), Value::Array(other_items)) => {
                if this_items.len() != other_items.len() {
                    found.push(format!(
                        "field={}.length this={} other={}",
                        name,
                        this_items.len(),
                        other_items.len()
                    ));
                }
                for (index, (this_item, other_item)) in
                    this_items.iter().zip(other_items).enumerate()
                {
                    if this_item != other_item {
                        found.push(format!(
                            "field={name}[{index}] this={this_item} other={other_item}"
                        ));
                    }
                }
            }
            _ if this_value != other_value => found.push(format!(
                "field={name} this={this_value} other={other_value}"
            )),
            _ => {}
        }
    }
    if found.is_empty() {
        found.push("field=line this=<the lines differ in their text only>".to_owned());
    }
    found
}

/// What a replay's checks counted, for its last lines. A block is counted once it has passed
/// the checks that were asked for and its line, if a report is written, is written; a
/// proposal once its answer was taken.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct ChecksSummary {
    /// `ProcessProposal` requests this run answered
    pub(super) proposals: u64,
    /// Of those, the ones whose answer was held against the log's answer and found equal
    pub(super) proposals_held_against_log: u64,
    /// The ones the log has no answer for that is certainly theirs
    pub(super) proposals_without_logged_answer: u64,
    /// The ones whose logged answer could not be read
    pub(super) proposals_with_unreadable_logged_answer: u64,
    /// Blocks finalized by this run
    pub(super) blocks: u64,
    pub(super) first_height: Option<u64>,
    pub(super) last_height: Option<u64>,
    /// State transitions in those blocks, where the replay has their results
    pub(super) transitions: u64,
    /// Blocks whose finalized proposal's answer was held against the log's and found equal
    pub(super) blocks_held_against_log: u64,
    /// Blocks for which the replay holds no answer of its own to take for theirs: the request
    /// before `FinalizeBlock`, `info` requests aside, was not the accepted proposal of that
    /// block
    pub(super) blocks_without_proposal: u64,
    /// Blocks held against the other run's report and found equal
    pub(super) blocks_held_against_report: u64,
    /// `info` requests passed over: logged by a Tenderdash of another ABCI version
    pub(super) info_passed_over: u64,
    /// The heights after which GroveDB's root hash was not the app hash
    pub(super) root_not_app_hash: Vec<u64>,
}

/// The `ProcessProposal` request the replay answered.
#[derive(Debug)]
pub(super) struct Proposed<'a> {
    pub(super) height: u64,
    pub(super) round: u32,
    /// The hash of the proposed block
    pub(super) block_hash: &'a [u8],
    pub(super) core_height: u32,
}

/// The answer to a `ProcessProposal`, kept until the next request that is not `Info`.
#[derive(Debug)]
struct Proposal {
    height: u64,
    round: u32,
    block_hash: Vec<u8>,
    accepted: bool,
    core_height: u32,
    tx: Vec<TxOutcome>,
    held_against_log: bool,
    sha256: String,
}

/// What a block left, as the replay reads it after `FinalizeBlock`.
#[derive(Debug)]
pub(super) struct Finalized<'a> {
    pub(super) height: u64,
    pub(super) round: u32,
    /// The hash of the finalized block
    pub(super) block_hash: &'a [u8],
    /// How many state transitions the finalized block has
    pub(super) transactions: usize,
    pub(super) protocol_version: u32,
    pub(super) app_hash: &'a [u8],
    pub(super) grove_root: &'a [u8],
}

/// The checks of a replay beyond the app hash: its report, and what it is held against.
#[derive(Debug, Default)]
pub(super) struct ReplayChecks {
    report: Option<ReportWriter>,
    other: Option<ReportReader>,
    core_answers: Option<CoreAnswers>,
    proposal: Option<Proposal>,
    summary: ChecksSummary,
}

fn status_name(status: i32) -> &'static str {
    match ProposalStatus::try_from(status) {
        Ok(ProposalStatus::Accept) => "Accepted",
        Ok(ProposalStatus::Reject) => "Rejected",
        _ => "Unknown",
    }
}

impl ReplayChecks {
    /// `report` is written, `other` is another run's report that every block is held
    /// against, and `database` is the block the replay's database is at, `None` for a new
    /// database.
    ///
    /// A block is held against `other` before its line is written, so a comparison has a
    /// report of its own: `other` is taken only with `report`. Where `report` has lines, of an
    /// earlier run on a database that is at the report's last block with its app hash, those
    /// lines are held against `other` here, before the replay goes on: the earlier run may
    /// have been held against another report or none.
    pub(super) fn open(
        report: Option<&Path>,
        other: Option<&Path>,
        database: Option<&DatabaseAt>,
    ) -> Result<Self, String> {
        const ITSELF: &str =
            "a report cannot be held against itself: --report and --compare-report name one file";
        let (Some(report_path), Some(other_path)) = (report, other) else {
            if other.is_some() {
                return Err(
                    "--compare-report needs --report: a block is held against the other report before its own line is written"
                        .to_owned(),
                );
            }
            return Ok(ReplayChecks {
                report: report
                    .map(|path| ReportWriter::open(path, database))
                    .transpose()?,
                ..Default::default()
            });
        };

        if report_path == other_path {
            return Err(ITSELF.to_owned());
        }
        // The file that is only read is opened first, so that a missing one is found before
        // the report is made
        let (mut other, other_identity) = ReportReader::open(other_path)?;
        let report = ReportWriter::open(report_path, database)?;
        let tell_error = |error: io::Error| {
            format!(
                "cannot tell report {} from report {}: {}",
                report_path.display(),
                other_path.display(),
                error
            )
        };
        // The two files that are open, whatever their names say by now
        let mut earlier = report.file.try_clone().map_err(tell_error)?;
        let same = earlier
            .try_clone()
            .and_then(Handle::from_file)
            .map_err(tell_error)?
            == other_identity;
        if same {
            return Err(ITSELF.to_owned());
        }

        // What an earlier run wrote, read from the file that was checked
        earlier
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("cannot read report {}: {}", report_path.display(), error))?;
        for line in records(BufReader::new(earlier)) {
            let (_, whole) = line.map_err(|error| {
                format!("cannot read report {}: {}", report_path.display(), error)
            })?;
            let (line, record) = whole.ok_or_else(|| {
                format!(
                    "report {} has a line that is not a report's",
                    report_path.display()
                )
            })?;
            other.hold(record.height, &line)?;
        }

        Ok(ReplayChecks {
            report: Some(report),
            other: Some(other),
            ..Default::default()
        })
    }

    /// Forgets the answer kept for the block being replayed. The replay calls it before
    /// every request that is neither `FinalizeBlock` nor `Info`: the answer to a proposal is
    /// taken for the finalized block's only where no request was executed between the two
    /// that could have replaced what the proposal's execution left for `FinalizeBlock` to
    /// commit.
    pub(super) fn request_begins(&mut self) {
        self.proposal = None;
    }

    /// Names the file Core's answers come from or go to. A request it failed on ends the
    /// replay before another block is counted, written or held.
    pub(super) fn with_core_answers(mut self, core_answers: Option<CoreAnswers>) -> Self {
        self.core_answers = core_answers;
        self
    }

    pub(super) fn summary(&self) -> &ChecksSummary {
        &self.summary
    }

    /// Notes an `info` request that was not replayed.
    pub(super) fn info_passed_over(&mut self) {
        self.summary.info_passed_over += 1;
    }

    /// Why Core's answers can no longer be trusted, if they cannot. Block execution may read
    /// a failed request to Core as Core being away for a moment and go on, so this is asked
    /// after every request, whatever the request made of the failure.
    pub(super) fn core_answers_failed(&self) -> Result<(), String> {
        match self.core_answers.as_ref().and_then(CoreAnswers::failure) {
            Some(failure) => Err(format!("Core's answers failed the replay: {failure}")),
            None => Ok(()),
        }
    }

    /// Takes the replay's answer to a proposal and holds it against the answer of the node
    /// that wrote the log, where the log has it.
    pub(super) fn proposal_answered(
        &mut self,
        proposed: Proposed,
        response: &ResponseProcessProposal,
        logged: &LoggedAnswer,
    ) -> Result<(), String> {
        self.core_answers_failed()?;
        let height = proposed.height;
        let tx: Vec<TxOutcome> = response
            .tx_results
            .iter()
            .map(|result| TxOutcome(result.code, result.gas_used))
            .collect();

        let held_against_log = match logged {
            LoggedAnswer::Absent => {
                self.summary.proposals_without_logged_answer += 1;
                false
            }
            LoggedAnswer::Unreadable(reason) => {
                tracing::warn!(
                    "the logged answer to process_proposal at height {} cannot be read: {}",
                    height,
                    reason
                );
                self.summary.proposals_with_unreadable_logged_answer += 1;
                false
            }
            LoggedAnswer::Proposal(logged) => {
                let found = log_differences(response, &tx, logged);
                if !found.is_empty() {
                    return Err(found
                        .iter()
                        .map(|difference| format!("CHAIN DIFFERS height={height} {difference}"))
                        .collect::<Vec<_>>()
                        .join("\n"));
                }
                self.summary.proposals_held_against_log += 1;
                true
            }
        };
        self.summary.proposals += 1;

        self.proposal = Some(Proposal {
            height,
            round: proposed.round,
            block_hash: proposed.block_hash.to_vec(),
            accepted: response.status == ProposalStatus::Accept as i32,
            core_height: proposed.core_height,
            tx,
            held_against_log,
            sha256: sha256_hex(response),
        });
        Ok(())
    }

    /// Takes what the block left once it is finalized: holds its line against the other
    /// run's, then writes it. A line that differs is not written, so a report never ends in
    /// a block that was found to differ.
    pub(super) fn block_finalized(
        &mut self,
        block: Finalized,
        response: &ResponseFinalizeBlock,
    ) -> Result<(), String> {
        self.core_answers_failed()?;
        // The answer kept is of the request before this one, `info` requests aside. It is the
        // block's only if that request was the very proposal that is finalized, and was
        // accepted.
        let proposal = self.proposal.take().filter(|proposal| {
            proposal.accepted
                && proposal.height == block.height
                && proposal.round == block.round
                && proposal.block_hash == block.block_hash
        });
        if let Some(proposal) = &proposal {
            if proposal.tx.len() != block.transactions {
                return Err(format!(
                    "the block at height {} has {} state transitions and the answer to its proposal has {} results",
                    block.height,
                    block.transactions,
                    proposal.tx.len()
                ));
            }
        }
        let transitions = proposal.as_ref().map_or(0, |proposal| proposal.tx.len());
        let held_against_log = proposal
            .as_ref()
            .is_some_and(|proposal| proposal.held_against_log);
        let (core_height, tx, proposal_sha256) = match proposal {
            Some(proposal) => (
                Some(proposal.core_height),
                Some(proposal.tx),
                Some(proposal.sha256),
            ),
            None => (None, None, None),
        };

        let record = BlockRecord {
            height: block.height,
            round: block.round,
            core_height,
            protocol_version: block.protocol_version,
            app_hash: hex::encode(block.app_hash),
            grove_root: hex::encode(block.grove_root),
            root_is_app_hash: block.app_hash == block.grove_root,
            tx,
            held_against_log,
            proposal_sha256,
            finalize_sha256: sha256_hex(response),
        };
        let line = serde_json::to_string(&record)
            .map_err(|error| format!("cannot write the report line: {error}"))?;

        if let Some(other) = &mut self.other {
            other.hold(record.height, &line)?;
        }
        if let Some(report) = &mut self.report {
            report.write(&line)?;
        }

        self.summary.blocks += 1;
        self.summary.first_height.get_or_insert(record.height);
        self.summary.last_height = Some(record.height);
        self.summary.transitions += transitions as u64;
        if record.tx.is_none() {
            self.summary.blocks_without_proposal += 1;
        }
        if record.held_against_log {
            self.summary.blocks_held_against_log += 1;
        }
        if self.other.is_some() {
            self.summary.blocks_held_against_report += 1;
        }
        if !record.root_is_app_hash {
            self.summary.root_not_app_hash.push(record.height);
        }
        Ok(())
    }
}

/// Where the replay's answer to a proposal is not the logged one, each as
/// `field=<name> this=<value> chain=<value>`.
fn log_differences(
    response: &ResponseProcessProposal,
    tx: &[TxOutcome],
    logged: &ChainProposalAnswer,
) -> Vec<String> {
    let mut found = Vec::new();
    let status = status_name(response.status);
    if status != logged.status {
        found.push(format!(
            "field=status this={} chain={}",
            status, logged.status
        ));
    }
    let app_hash = hex::encode(&response.app_hash);
    if !app_hash.eq_ignore_ascii_case(&logged.app_hash) {
        found.push(format!(
            "field=app_hash this={} chain={}",
            app_hash, logged.app_hash
        ));
    }
    if tx.len() != logged.tx.len() {
        found.push(format!(
            "field=tx.length this={} chain={}",
            tx.len(),
            logged.tx.len()
        ));
    }
    for (index, (ours, theirs)) in tx.iter().zip(&logged.tx).enumerate() {
        if ours != theirs {
            found.push(format!(
                "field=tx[{}] this=[{},{}] chain=[{},{}]",
                index, ours.0, ours.1, theirs.0, theirs.1
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::ops::RangeInclusive;
    use tenderdash_abci::proto::abci::ExecTxResult;

    const APP_HASH: [u8; 32] = [0x26; 32];
    const OTHER_ROOT: [u8; 32] = [0xc7; 32];

    fn tx_result(code: u32, gas_used: i64) -> ExecTxResult {
        ExecTxResult {
            code,
            gas_used,
            ..Default::default()
        }
    }

    fn proposal_answer(tx: &[(u32, i64)]) -> ResponseProcessProposal {
        ResponseProcessProposal {
            status: ProposalStatus::Accept as i32,
            app_hash: APP_HASH.to_vec(),
            tx_results: tx
                .iter()
                .map(|(code, gas_used)| tx_result(*code, *gas_used))
                .collect(),
            ..Default::default()
        }
    }

    fn logged_answer(tx: &[(u32, i64)]) -> LoggedAnswer {
        LoggedAnswer::Proposal(ChainProposalAnswer {
            status: "Accepted".to_owned(),
            app_hash: hex::encode(APP_HASH),
            tx: tx
                .iter()
                .map(|(code, gas_used)| TxOutcome(*code, *gas_used))
                .collect(),
        })
    }

    /// The hash of the block the tests propose and finalize at a height.
    fn block_hash(height: u64) -> Vec<u8> {
        vec![height as u8; 32]
    }

    fn proposed(height: u64, round: u32, block_hash: &[u8]) -> Proposed<'_> {
        Proposed {
            height,
            round,
            block_hash,
            core_height: 1_090_000 + height as u32,
        }
    }

    fn finalized<'a>(
        height: u64,
        block_hash: &'a [u8],
        transactions: usize,
        grove_root: &'a [u8],
    ) -> Finalized<'a> {
        Finalized {
            height,
            round: 0,
            block_hash,
            transactions,
            protocol_version: 9,
            app_hash: &APP_HASH,
            grove_root,
        }
    }

    /// Replays the blocks `heights` with one state transition each, whose fee is `gas_used`.
    fn replay_blocks(
        checks: &mut ReplayChecks,
        heights: RangeInclusive<u64>,
        gas_used: i64,
    ) -> Result<(), String> {
        for height in heights {
            let hash = block_hash(height);
            checks.proposal_answered(
                proposed(height, 0, &hash),
                &proposal_answer(&[(0, gas_used)]),
                &logged_answer(&[(0, gas_used)]),
            )?;
            checks.block_finalized(
                finalized(height, &hash, 1, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )?;
        }
        Ok(())
    }

    /// The database after the tests' block at `height`.
    fn database(height: u64) -> DatabaseAt {
        DatabaseAt {
            height,
            app_hash: APP_HASH.to_vec(),
        }
    }

    fn report_lines(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn should_write_one_line_a_block_with_nothing_of_the_run_in_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&path), None, None).unwrap();

        let hash = block_hash(1);
        checks
            .proposal_answered(
                proposed(1, 0, &hash),
                &proposal_answer(&[(0, 117438280), (40212, 0)]),
                &logged_answer(&[(0, 117438280), (40212, 0)]),
            )
            .unwrap();
        checks
            .block_finalized(
                finalized(1, &hash, 2, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        // The log has no answer for the second block, which leaves another root
        let hash = block_hash(2);
        checks
            .proposal_answered(
                proposed(2, 0, &hash),
                &proposal_answer(&[]),
                &LoggedAnswer::Absent,
            )
            .unwrap();
        checks
            .block_finalized(
                finalized(2, &hash, 0, &OTHER_ROOT),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();

        let proposal_one = sha256_hex(&proposal_answer(&[(0, 117438280), (40212, 0)]));
        let proposal_two = sha256_hex(&proposal_answer(&[]));
        let finalize = sha256_hex(&ResponseFinalizeBlock::default());
        let app_hash = "26".repeat(32);
        let other_root = "c7".repeat(32);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            format!(
                "{{\"height\":1,\"round\":0,\"core_height\":1090001,\"protocol_version\":9,\"app_hash\":\"{app_hash}\",\"grove_root\":\"{app_hash}\",\"root_is_app_hash\":true,\"tx\":[[0,117438280],[40212,0]],\"held_against_log\":true,\"proposal_sha256\":\"{proposal_one}\",\"finalize_sha256\":\"{finalize}\"}}\n\
                 {{\"height\":2,\"round\":0,\"core_height\":1090002,\"protocol_version\":9,\"app_hash\":\"{app_hash}\",\"grove_root\":\"{other_root}\",\"root_is_app_hash\":false,\"tx\":[],\"held_against_log\":false,\"proposal_sha256\":\"{proposal_two}\",\"finalize_sha256\":\"{finalize}\"}}\n"
            )
        );
        assert_ne!(proposal_one, proposal_two);
        assert_eq!(
            *checks.summary(),
            ChecksSummary {
                proposals: 2,
                proposals_held_against_log: 1,
                proposals_without_logged_answer: 1,
                proposals_with_unreadable_logged_answer: 0,
                blocks: 2,
                first_height: Some(1),
                last_height: Some(2),
                transitions: 2,
                blocks_held_against_log: 1,
                blocks_without_proposal: 0,
                blocks_held_against_report: 0,
                info_passed_over: 0,
                root_not_app_hash: vec![2],
            }
        );
    }

    #[test]
    fn should_stop_at_a_fee_that_is_not_the_logged_fee() {
        let mut checks = ReplayChecks::default();
        let hash = block_hash(7);
        let error = checks
            .proposal_answered(
                proposed(7, 0, &hash),
                &proposal_answer(&[(0, 100), (0, 201), (0, 300)]),
                &logged_answer(&[(0, 100), (0, 200), (0, 300)]),
            )
            .unwrap_err();
        assert_eq!(
            error,
            "CHAIN DIFFERS height=7 field=tx[1] this=[0,201] chain=[0,200]"
        );
        assert_eq!(*checks.summary(), ChecksSummary::default());
        // Nothing is kept of an answer that differs
        checks
            .block_finalized(
                finalized(7, &hash, 3, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        assert_eq!(checks.summary().blocks_without_proposal, 1);
    }

    #[test]
    fn should_stop_at_every_part_of_an_answer_that_is_not_the_logged_answer() {
        let mut checks = ReplayChecks::default();
        let hash = block_hash(7);
        let mut response = proposal_answer(&[(0, 100), (0, 200)]);
        response.status = ProposalStatus::Reject as i32;
        response.app_hash = OTHER_ROOT.to_vec();
        let error = checks
            .proposal_answered(
                proposed(7, 0, &hash),
                &response,
                &logged_answer(&[(1000, 100)]),
            )
            .unwrap_err();
        assert_eq!(
            error,
            format!(
                "CHAIN DIFFERS height=7 field=status this=Rejected chain=Accepted\n\
                 CHAIN DIFFERS height=7 field=app_hash this={} chain={}\n\
                 CHAIN DIFFERS height=7 field=tx.length this=2 chain=1\n\
                 CHAIN DIFFERS height=7 field=tx[0] this=[0,100] chain=[1000,100]",
                "c7".repeat(32),
                "26".repeat(32)
            )
        );

        assert_eq!(status_name(0), "Unknown");
        assert_eq!(status_name(77), "Unknown");
    }

    #[test]
    fn should_count_a_logged_answer_it_cannot_read_and_say_so_in_the_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&path), None, None).unwrap();
        let hash = block_hash(7);
        checks
            .proposal_answered(
                proposed(7, 0, &hash),
                &proposal_answer(&[(0, 100)]),
                &LoggedAnswer::Unreadable("a string does not end".to_owned()),
            )
            .unwrap();
        checks
            .block_finalized(
                finalized(7, &hash, 1, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        assert_eq!(checks.summary().proposals_with_unreadable_logged_answer, 1);
        assert_eq!(checks.summary().proposals_held_against_log, 0);
        assert_eq!(checks.summary().blocks_held_against_log, 0);
        assert_eq!(checks.summary().blocks, 1);
        let lines = report_lines(&path);
        assert!(
            lines[0].contains("\"tx\":[[0,100]],\"held_against_log\":false,"),
            "{}",
            lines[0]
        );
    }

    #[test]
    fn should_count_a_block_as_held_against_the_log_once_whatever_rounds_it_took() {
        let mut checks = ReplayChecks::default();
        let refused = block_hash(40);
        let accepted = block_hash(41);
        // Round 0 is refused by the node and by the replay alike, round 1 is the block
        let mut response = proposal_answer(&[(0, 100)]);
        response.status = ProposalStatus::Reject as i32;
        let mut logged = ChainProposalAnswer {
            status: "Rejected".to_owned(),
            app_hash: hex::encode(APP_HASH),
            tx: vec![TxOutcome(0, 100)],
        };
        checks
            .proposal_answered(
                proposed(4, 0, &refused),
                &response,
                &LoggedAnswer::Proposal(logged.clone()),
            )
            .unwrap();
        logged.status = "Accepted".to_owned();
        checks
            .proposal_answered(
                proposed(4, 1, &accepted),
                &proposal_answer(&[(0, 100)]),
                &LoggedAnswer::Proposal(logged),
            )
            .unwrap();
        checks
            .block_finalized(
                Finalized {
                    round: 1,
                    ..finalized(4, &accepted, 1, &APP_HASH)
                },
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        assert_eq!(checks.summary().proposals, 2);
        assert_eq!(checks.summary().proposals_held_against_log, 2);
        assert_eq!(checks.summary().blocks, 1);
        assert_eq!(checks.summary().blocks_held_against_log, 1);
        assert_eq!(checks.summary().blocks_without_proposal, 0);
    }

    #[test]
    fn should_hold_every_block_against_another_run_report() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.jsonl");
        let second = dir.path().join("second.jsonl");

        let mut checks = ReplayChecks::open(Some(&first), None, None).unwrap();
        replay_blocks(&mut checks, 1..=5, 100).unwrap();
        assert_eq!(checks.summary().blocks_held_against_report, 0);

        let mut checks = ReplayChecks::open(Some(&second), Some(&first), None).unwrap();
        replay_blocks(&mut checks, 1..=5, 100).unwrap();
        assert_eq!(checks.summary().blocks_held_against_report, 5);
        assert_eq!(checks.summary().blocks_held_against_log, 5);
        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
    }

    #[test]
    fn should_stop_at_the_first_block_that_differs_and_not_write_its_line() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.jsonl");
        let mut checks = ReplayChecks::open(Some(&first), None, None).unwrap();
        replay_blocks(&mut checks, 1..=3, 100).unwrap();

        let second = dir.path().join("second.jsonl");
        let mut checks = ReplayChecks::open(Some(&second), Some(&first), None).unwrap();
        replay_blocks(&mut checks, 1..=2, 100).unwrap();
        // The third block pays another fee, has a second transition and leaves another root
        let hash = block_hash(3);
        checks
            .proposal_answered(
                proposed(3, 0, &hash),
                &proposal_answer(&[(0, 101), (0, 5)]),
                &LoggedAnswer::Absent,
            )
            .unwrap();
        let error = checks
            .block_finalized(
                finalized(3, &hash, 2, &OTHER_ROOT),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap_err();

        let lines: Vec<&str> = error.lines().collect();
        let other_line = report_lines(&first)[2].clone();
        let this_line = lines[lines.len() - 2]
            .strip_prefix("DIFFERS height=3 this=")
            .expect("the line of this replay is given")
            .to_owned();
        assert!(
            this_line.contains("\"tx\":[[0,101],[0,5]],\"held_against_log\":false,"),
            "{this_line}"
        );
        assert_eq!(
            lines,
            vec![
                format!(
                    "DIFFERS height=3 field=grove_root this=\"{}\" other=\"{}\"",
                    "c7".repeat(32),
                    "26".repeat(32)
                ),
                "DIFFERS height=3 field=root_is_app_hash this=false other=true".to_owned(),
                "DIFFERS height=3 field=tx.length this=2 other=1".to_owned(),
                "DIFFERS height=3 field=tx[0] this=[0,101] other=[0,100]".to_owned(),
                "DIFFERS height=3 field=held_against_log this=false other=true".to_owned(),
                format!(
                    "DIFFERS height=3 field=proposal_sha256 this=\"{}\" other=\"{}\"",
                    sha256_hex(&proposal_answer(&[(0, 101), (0, 5)])),
                    sha256_hex(&proposal_answer(&[(0, 100)]))
                ),
                format!("DIFFERS height=3 this={this_line}"),
                format!("DIFFERS height=3 other={other_line}"),
            ]
        );
        // The block that differs is neither written nor counted
        assert_eq!(report_lines(&second).len(), 2);
        assert_eq!(checks.summary().blocks_held_against_report, 2);
        assert_eq!(checks.summary().blocks, 2);
        assert_eq!(checks.summary().last_height, Some(2));
        assert_eq!(checks.summary().root_not_app_hash, Vec::<u64>::new());
    }

    #[test]
    fn should_take_the_other_report_from_the_block_this_replay_starts_at() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.jsonl");
        let mut checks = ReplayChecks::open(Some(&first), None, None).unwrap();
        replay_blocks(&mut checks, 1..=6, 100).unwrap();

        // This replay's database and report are at the third block
        let second = dir.path().join("second.jsonl");
        let mut checks = ReplayChecks::open(Some(&second), None, None).unwrap();
        replay_blocks(&mut checks, 1..=3, 100).unwrap();
        drop(checks);
        let mut checks =
            ReplayChecks::open(Some(&second), Some(&first), Some(&database(3))).unwrap();
        replay_blocks(&mut checks, 4..=6, 100).unwrap();
        assert_eq!(checks.summary().blocks_held_against_report, 3);
        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());

        // The other report ends before this replay does
        let error = replay_blocks(&mut checks, 7..=7, 100).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} has no line for height 7: it ends before",
                first.display()
            )
        );
        assert_eq!(checks.summary().blocks, 3);
        assert_eq!(report_lines(&second).len(), 6);
    }

    #[test]
    fn should_take_the_lines_of_the_other_report_one_after_the_other() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.jsonl");
        let mut checks = ReplayChecks::open(Some(&first), None, None).unwrap();
        replay_blocks(&mut checks, 1..=4, 100).unwrap();
        drop(checks);
        let lines = report_lines(&first);
        let report = dir.path().join("report.jsonl");

        let error =
            ReplayChecks::open(Some(&report), Some(&dir.path().join("missing")), None).unwrap_err();
        assert!(error.starts_with("cannot open report "), "{error}");
        // The file that is only read is opened first: nothing was made
        assert!(!report.exists());

        // The other report starts after this replay's first block
        let other = dir.path().join("other.jsonl");
        fs::write(&other, format!("{}\n{}\n", lines[2], lines[3])).unwrap();
        let mut checks = ReplayChecks::open(Some(&report), Some(&other), None).unwrap();
        let error = replay_blocks(&mut checks, 1..=1, 100).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} has no line for height 1: line 1 is for height 3",
                other.display()
            )
        );

        // The other report's lines are taken one after the other: here the second block's
        // line is there twice, the second time with another fee
        let mut twice = lines.clone();
        twice.insert(2, lines[1].replace("[0,100]", "[0,999]"));
        fs::write(&other, twice.join("\n") + "\n").unwrap();
        fs::remove_file(&report).unwrap();
        let mut checks = ReplayChecks::open(Some(&report), Some(&other), None).unwrap();
        replay_blocks(&mut checks, 1..=2, 100).unwrap();
        let error = replay_blocks(&mut checks, 3..=3, 100).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} has no line for height 3: line 3 is for height 2",
                other.display()
            )
        );

        fs::write(&other, format!("{}\nnot a line\n", lines[0])).unwrap();
        fs::remove_file(&report).unwrap();
        let mut checks = ReplayChecks::open(Some(&report), Some(&other), None).unwrap();
        replay_blocks(&mut checks, 1..=1, 100).unwrap();
        let error = replay_blocks(&mut checks, 2..=2, 100).unwrap_err();
        assert_eq!(
            error,
            format!(
                "line 2 of report {} is not a whole line of a report",
                other.display()
            )
        );

        // The other report's last line has no line end: it may be half of a longer one
        fs::write(&other, lines[..2].join("\n")).unwrap();
        fs::remove_file(&report).unwrap();
        let mut checks = ReplayChecks::open(Some(&report), Some(&other), None).unwrap();
        replay_blocks(&mut checks, 1..=1, 100).unwrap();
        let error = replay_blocks(&mut checks, 2..=2, 100).unwrap_err();
        assert!(error.starts_with("line 2 of report "), "{error}");
    }

    #[test]
    fn should_hold_a_report_cut_at_its_front_against_no_other() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.jsonl");
        let mut checks = ReplayChecks::open(Some(&first), None, None).unwrap();
        replay_blocks(&mut checks, 1..=4, 100).unwrap();
        drop(checks);

        // Only the last line of an earlier run's report is left, and it is the database's
        let cut = dir.path().join("cut.jsonl");
        fs::write(&cut, format!("{}\n", report_lines(&first)[3])).unwrap();
        let error = ReplayChecks::open(Some(&cut), Some(&first), Some(&database(4))).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} has no line for height 4: line 1 is for height 1",
                first.display()
            )
        );
    }

    #[test]
    fn should_hold_a_block_against_another_report_only_with_a_report_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other.jsonl");
        fs::write(&other, "").unwrap();
        let error = ReplayChecks::open(None, Some(&other), None).unwrap_err();
        assert_eq!(
            error,
            "--compare-report needs --report: a block is held against the other report before its own line is written"
        );
    }

    #[test]
    fn should_add_to_a_report_only_from_the_block_its_database_is_at() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&path), None, None).unwrap();
        replay_blocks(&mut checks, 1..=3, 100).unwrap();
        drop(checks);

        let error = ReplayChecks::open(Some(&path), None, Some(&database(2))).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} ends at height 3, and the database is at height 2; a report goes on only from the block its database is at",
                path.display()
            )
        );
        let error = ReplayChecks::open(Some(&path), None, None).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} ends at height 3, and the database is new; a report goes on only from the block its database is at",
                path.display()
            )
        );
        // A database at the same height that left another app hash there
        let another = DatabaseAt {
            height: 3,
            app_hash: OTHER_ROOT.to_vec(),
        };
        let error = ReplayChecks::open(Some(&path), None, Some(&another)).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} ends at height 3 with app hash {}, and the database has app hash {} there; the report is of another database",
                path.display(),
                "26".repeat(32),
                "c7".repeat(32)
            )
        );

        let mut checks = ReplayChecks::open(Some(&path), None, Some(&database(3))).unwrap();
        replay_blocks(&mut checks, 4..=5, 100).unwrap();
        drop(checks);
        let heights: Vec<u64> = report_lines(&path)
            .iter()
            .map(|line| record_of(line).unwrap().height)
            .collect();
        assert_eq!(heights, vec![1, 2, 3, 4, 5]);

        // A report with no lines is not the report of a database that has blocks: the line
        // of its first block may have been found to differ and not been written
        let empty = dir.path().join("empty.jsonl");
        let error = ReplayChecks::open(Some(&empty), None, Some(&database(5))).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} has no lines, and the database is at height 5; a report is written from a new database on",
                empty.display()
            )
        );

        let error = ReplayChecks::open(Some(dir.path()), None, None).unwrap_err();
        assert!(error.starts_with("cannot open report "), "{error}");
    }

    #[test]
    fn should_refuse_a_report_whose_lines_are_not_whole_or_not_one_block_after_the_other() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&path), None, None).unwrap();
        replay_blocks(&mut checks, 1..=5, 100).unwrap();
        drop(checks);
        let whole = fs::read_to_string(&path).unwrap();
        let lines = report_lines(&path);

        // A report that a stopped replay left with half a line
        fs::write(&path, format!("{whole}{{\"height\":6,\"rou")).unwrap();
        let error = ReplayChecks::open(Some(&path), None, Some(&database(5))).unwrap_err();
        assert_eq!(
            error,
            format!(
                "line 6 of report {} is not a whole line of a report",
                path.display()
            )
        );
        // A whole line with no line end is half a line too
        fs::write(&path, format!("{whole}{}", lines[4].replace(":5,", ":6,"))).unwrap();
        let error = ReplayChecks::open(Some(&path), None, Some(&database(6))).unwrap_err();
        assert!(error.starts_with("line 6 of report "), "{error}");
        // Bytes cut inside a character
        let mut bytes = whole.clone().into_bytes();
        bytes.extend_from_slice(&[0xc3]);
        fs::write(&path, bytes).unwrap();
        let error = ReplayChecks::open(Some(&path), None, Some(&database(5))).unwrap_err();
        assert!(error.starts_with("line 6 of report "), "{error}");
        // A line that has a height and is no line of a report
        fs::write(&path, format!("{whole}{{\"height\":6}}\n")).unwrap();
        let error = ReplayChecks::open(Some(&path), None, Some(&database(6))).unwrap_err();
        assert!(error.starts_with("line 6 of report "), "{error}");

        // A line is missing in the middle, and the report still ends where the database is
        fs::write(
            &path,
            format!("{}\n{}\n{}\n{}\n", lines[0], lines[1], lines[3], lines[4]),
        )
        .unwrap();
        let error = ReplayChecks::open(Some(&path), None, Some(&database(5))).unwrap_err();
        assert_eq!(
            error,
            format!(
                "line 3 of report {} is for height 4 and does not follow the line before it",
                path.display()
            )
        );
        // The greatest height there is has no line after it
        let last = lines[4].replace("\"height\":5,", "\"height\":18446744073709551615,");
        let first = lines[0].replace("\"height\":1,", "\"height\":0,");
        fs::write(&path, format!("{last}\n{first}\n")).unwrap();
        let error = ReplayChecks::open(Some(&path), None, Some(&database(0))).unwrap_err();
        assert!(
            error.contains("is for height 0 and does not follow the line before it"),
            "{error}"
        );
    }

    #[test]
    fn should_hold_the_lines_an_earlier_run_wrote_against_the_other_report_before_going_on() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.jsonl");
        let mut checks = ReplayChecks::open(Some(&first), None, None).unwrap();
        replay_blocks(&mut checks, 1..=6, 100).unwrap();

        // An earlier run of the second replay was held against nothing, and paid another
        // fee in its third block
        let second = dir.path().join("second.jsonl");
        let mut checks = ReplayChecks::open(Some(&second), None, None).unwrap();
        replay_blocks(&mut checks, 1..=2, 100).unwrap();
        replay_blocks(&mut checks, 3..=3, 101).unwrap();
        replay_blocks(&mut checks, 4..=4, 100).unwrap();
        drop(checks);

        let error =
            ReplayChecks::open(Some(&second), Some(&first), Some(&database(4))).unwrap_err();
        assert!(
            error.starts_with("DIFFERS height=3 field=tx[0] this=[0,101] other=[0,100]\n"),
            "{error}"
        );

        // With equal lines the replay goes on, and holds the next block against the next line
        let third = dir.path().join("third.jsonl");
        let mut checks = ReplayChecks::open(Some(&third), None, None).unwrap();
        replay_blocks(&mut checks, 1..=4, 100).unwrap();
        drop(checks);
        let mut checks =
            ReplayChecks::open(Some(&third), Some(&first), Some(&database(4))).unwrap();
        replay_blocks(&mut checks, 5..=6, 100).unwrap();
        assert_eq!(checks.summary().blocks_held_against_report, 2);
        assert_eq!(fs::read(&first).unwrap(), fs::read(&third).unwrap());

        // The other report is shorter than what the earlier run wrote
        let short = dir.path().join("short.jsonl");
        fs::write(&short, report_lines(&first)[..2].join("\n") + "\n").unwrap();
        let error = ReplayChecks::open(Some(&third), Some(&short), Some(&database(6))).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} has no line for height 3: it ends before",
                short.display()
            )
        );
    }

    #[test]
    fn should_not_let_a_block_that_differs_pass_at_a_second_start() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.jsonl");
        let mut checks = ReplayChecks::open(Some(&first), None, None).unwrap();
        replay_blocks(&mut checks, 1..=3, 100).unwrap();

        // The very first block of the second replay differs: its line is not written, and
        // its database is at the first block all the same
        let second = dir.path().join("second.jsonl");
        let mut checks = ReplayChecks::open(Some(&second), Some(&first), None).unwrap();
        let error = replay_blocks(&mut checks, 1..=1, 101).unwrap_err();
        assert!(error.starts_with("DIFFERS height=1 "), "{error}");
        drop(checks);
        assert_eq!(fs::read_to_string(&second).unwrap(), "");

        let error =
            ReplayChecks::open(Some(&second), Some(&first), Some(&database(1))).unwrap_err();
        assert_eq!(
            error,
            format!(
                "report {} has no lines, and the database is at height 1; a report is written from a new database on",
                second.display()
            )
        );
    }

    #[test]
    fn should_write_no_proposal_answer_for_a_block_that_is_not_the_one_last_proposed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&path), None, None).unwrap();
        let finalize = ResponseFinalizeBlock::default();
        let mut height = 3;
        let mut next = |checks: &mut ReplayChecks, proposal: Option<(u32, Vec<u8>, bool)>| {
            height += 1;
            let hash = block_hash(height);
            if let Some((round, proposed_hash, accepted)) = proposal {
                let mut response = proposal_answer(&[(0, 100)]);
                if !accepted {
                    response.status = ProposalStatus::Reject as i32;
                }
                checks
                    .proposal_answered(
                        proposed(height, round, &proposed_hash),
                        &response,
                        &LoggedAnswer::Absent,
                    )
                    .unwrap();
            }
            checks
                .block_finalized(finalized(height, &hash, 1, &APP_HASH), &finalize)
                .unwrap();
        };

        // The last proposal the replay answered was of another round
        next(&mut checks, Some((1, block_hash(4), true)));
        // Of the same round and another block: another hash
        next(&mut checks, Some((0, block_hash(99), true)));
        // Of the same round and block, and refused
        next(&mut checks, Some((0, block_hash(6), false)));
        // No proposal at all
        next(&mut checks, None);
        // And the proposal that was finalized
        next(&mut checks, Some((0, block_hash(8), true)));

        let lines = report_lines(&path);
        let app_hash = "26".repeat(32);
        let finalize = sha256_hex(&finalize);
        assert_eq!(
            lines[0],
            format!(
                "{{\"height\":4,\"round\":0,\"core_height\":null,\"protocol_version\":9,\"app_hash\":\"{app_hash}\",\"grove_root\":\"{app_hash}\",\"root_is_app_hash\":true,\"tx\":null,\"held_against_log\":false,\"proposal_sha256\":null,\"finalize_sha256\":\"{finalize}\"}}"
            )
        );
        assert_eq!(lines.len(), 5);
        for line in &lines[..4] {
            assert!(line.contains("\"core_height\":null,"), "{line}");
            assert!(line.contains("\"tx\":null,"), "{line}");
            assert!(line.contains("\"proposal_sha256\":null,"), "{line}");
        }
        assert!(lines[4].contains("\"tx\":[[0,100]],"), "{}", lines[4]);
        assert_eq!(checks.summary().blocks_without_proposal, 4);
        assert_eq!(checks.summary().transitions, 1);
        assert_eq!(checks.summary().blocks, 5);
        assert_eq!(checks.summary().proposals, 4);
    }

    #[test]
    fn should_use_the_answer_to_a_proposal_for_one_block_only() {
        let mut checks = ReplayChecks::default();
        let hash = block_hash(5);
        checks
            .proposal_answered(
                proposed(5, 0, &hash),
                &proposal_answer(&[]),
                &LoggedAnswer::Absent,
            )
            .unwrap();
        // A block of another height takes the kept answer away with it
        checks
            .block_finalized(
                finalized(6, &block_hash(6), 0, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        checks
            .block_finalized(
                finalized(5, &hash, 0, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        assert_eq!(checks.summary().blocks_without_proposal, 2);
    }

    #[test]
    fn should_stop_where_a_block_has_more_state_transitions_than_its_answer_has_results() {
        let mut checks = ReplayChecks::default();
        let hash = block_hash(5);
        checks
            .proposal_answered(
                proposed(5, 0, &hash),
                &proposal_answer(&[(0, 100)]),
                &LoggedAnswer::Absent,
            )
            .unwrap();
        let error = checks
            .block_finalized(
                finalized(5, &hash, 2, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap_err();
        assert_eq!(
            error,
            "the block at height 5 has 2 state transitions and the answer to its proposal has 1 results"
        );
        assert_eq!(checks.summary().blocks, 0);
    }

    #[test]
    fn should_refuse_to_hold_a_report_against_itself_under_any_of_its_names() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.jsonl");
        const ITSELF: &str =
            "a report cannot be held against itself: --report and --compare-report name one file";

        let error = ReplayChecks::open(Some(&path), Some(&path), None).unwrap_err();
        assert_eq!(error, ITSELF);
        assert!(!path.exists());

        fs::write(&path, "").unwrap();
        // Another spelling of the path
        let spelled = dir.path().join(".").join("report.jsonl");
        let error = ReplayChecks::open(Some(&path), Some(&spelled), None).unwrap_err();
        assert_eq!(error, ITSELF);
        // A second name of the file
        let linked = dir.path().join("linked.jsonl");
        fs::hard_link(&path, &linked).unwrap();
        let error = ReplayChecks::open(Some(&linked), Some(&path), None).unwrap_err();
        assert_eq!(error, ITSELF);

        // Two files are two reports
        let other = dir.path().join("other.jsonl");
        fs::write(&other, "").unwrap();
        ReplayChecks::open(Some(&path), Some(&other), None).unwrap();
    }

    #[test]
    fn should_forget_the_answer_to_a_proposal_when_another_request_begins() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.jsonl");
        let mut checks = ReplayChecks::open(Some(&path), None, None).unwrap();
        let hash = block_hash(5);
        checks
            .proposal_answered(
                proposed(5, 0, &hash),
                &proposal_answer(&[(0, 100)]),
                &logged_answer(&[(0, 100)]),
            )
            .unwrap();
        // A proposal is prepared, say, before the block that was processed is finalized
        checks.request_begins();
        checks
            .block_finalized(
                finalized(5, &hash, 1, &APP_HASH),
                &ResponseFinalizeBlock::default(),
            )
            .unwrap();
        let lines = report_lines(&path);
        assert!(
            lines[0].contains("\"tx\":null,\"held_against_log\":false,"),
            "{}",
            lines[0]
        );
        assert_eq!(checks.summary().blocks_without_proposal, 1);
        assert_eq!(checks.summary().blocks_held_against_log, 0);
        assert_eq!(checks.summary().proposals_held_against_log, 1);
    }

    #[test]
    fn should_name_fields_only_one_line_has_and_lines_that_are_no_report() {
        assert_eq!(
            differences("{\"height\":1,\"a\":1}", "{\"height\":1,\"b\":[1]}"),
            vec![
                "field=a this=1 other=\"<absent>\"",
                "field=b this=\"<absent>\" other=[1]"
            ]
        );
        assert_eq!(
            differences("{\"height\":1, \"a\":1}", "{\"height\":1,\"a\":1}"),
            vec!["field=line this=<the lines differ in their text only>"]
        );
        assert_eq!(
            differences("{\"height\":1}", "not a line"),
            vec!["field=line this=<not a line of a report, or the other is not>"]
        );
        assert_eq!(record_of("[1]"), None);
        assert_eq!(record_of("{\"height\":1}"), None);
        // The fields that may be null have to be there
        let whole = "{\"height\":1,\"round\":0,\"core_height\":null,\"protocol_version\":9,\"app_hash\":\"\",\"grove_root\":\"\",\"root_is_app_hash\":true,\"tx\":null,\"held_against_log\":false,\"proposal_sha256\":null,\"finalize_sha256\":\"\"}";
        assert!(record_of(whole).is_some());
        assert_eq!(record_of(&whole.replace("\"tx\":null,", "")), None);
        assert_eq!(record_of(&whole.replace("\"core_height\":null,", "")), None);
        // A field given twice, the second time with the value the line has
        assert_eq!(
            record_of(&whole.replacen("\"height\":", "\"height\":99,\"height\":", 1)),
            None
        );
        // A list of the values in a line's order is no line
        assert_eq!(
            record_of("[1,0,null,9,\"\",\"\",true,null,false,null,\"\"]"),
            None
        );
        // A field a report line does not have
        let line = serde_json::to_string(&BlockRecord {
            height: 1,
            round: 0,
            core_height: None,
            protocol_version: 9,
            app_hash: String::new(),
            grove_root: String::new(),
            root_is_app_hash: true,
            tx: None,
            held_against_log: false,
            proposal_sha256: None,
            finalize_sha256: String::new(),
        })
        .unwrap();
        assert!(record_of(&line).is_some());
        assert_eq!(record_of(&line.replacen('{', "{\"more\":1,", 1)), None);
    }
}
