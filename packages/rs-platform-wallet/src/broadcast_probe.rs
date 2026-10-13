//! Asking Core nodes directly whether a signed transaction has landed.
//!
//! The P2P network has no negative signal — modern Dash Core removed BIP61
//! `reject` — so a transaction whose SPV broadcast saw no acceptance signal
//! (`BroadcastResult::Uncertain` → [`BroadcastError::MaybeSent`]) stays
//! ambiguous for as long as nobody asks.
//!
//! Core's `sendrawtransaction`, which DAPI's `broadcastTransaction` wraps, is
//! not silent. Measured against Dash Core 22.1.3 (testnet and mainnet,
//! 2026-09-24):
//!
//! | transaction                                   | Core answer                               |
//! |-----------------------------------------------|-------------------------------------------|
//! | already in the mempool                        | success, the txid                         |
//! | already in a block, an output still unspent   | `-27 Transaction already in block chain`  |
//! | already in a block, **every output spent**    | `-25 bad-txns-inputs-missingorspent`      |
//! | spends an input that does not exist           | `-25 bad-txns-inputs-missingorspent`      |
//! | spends an input an IS-locked tx already spent | `-26 tx-txlock-conflict`                  |
//!
//! The probe uses the positive answers: a node holding the transaction, in its
//! mempool or in a block, makes it accepted. A refusal never
//! settles anything. Proving that a transaction can never land would need the
//! finality of whatever spent its input, which these answers do not show — a
//! node lagging behind a parent's block, an ancestor still propagating, or a
//! competing spend in a block that can still be reorged all look the same. So
//! a refusal only sends the probe to ask whether the txid is known at all
//! (DAPI `getTransaction`, served from the evonode's transaction index): the
//! third row is a *mined* transaction whose outputs are all spent (measured
//! on testnet 2026-09-29: `e758f06a…`, block 1560020, both outputs spent →
//! `-25`), and the lookup is what finds it. Otherwise the transaction stays
//! unresolved, with the refusal in the reason.
//!
//! The probe does not tell "in a mempool" from "in a block": no host and no
//! schedule reads the difference, and a node's word on a block is not
//! settlement — the block may be on a stale fork or about to be reorged out.
//! The wallet's own block processing settles a send; until then the resolver
//! keeps probing an accepted one like any other.
//!
//! The two DAPI implementations surface those codes differently — the JS
//! server maps `-25` to `InvalidArgument`, rs-dapi to `FailedPrecondition` —
//! so the classification keys on Core's reason text, which both pass through
//! in the status message, and on `AlreadyExists` for `-27`, which both agree
//! on.
//!
//! [`BroadcastError::MaybeSent`]: crate::broadcaster::BroadcastError::MaybeSent

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dash_sdk::core::transaction_from_reply;
use dash_sdk::dapi_client::transport::TransportError;
use dash_sdk::dapi_client::{
    DapiClientError, DapiRequestExecutor, ExecutionResult, RequestSettings,
};
use dash_sdk::dapi_grpc::core::v0::{
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetTransactionRequest,
    GetTransactionResponse,
};
use dash_sdk::dapi_grpc::tonic::Code;
use dashcore::consensus;
use dashcore::{Transaction, Txid};
use tokio::time::timeout;

/// Core reject reasons about the transaction's inputs.
///
/// - `bad-txns-inputs-missingorspent`: an input is neither in the UTXO set
///   nor created by anything in this node's mempool. `Missing inputs` is the
///   older spelling of the same check. Also what a mined transaction whose
///   outputs are all spent gets.
/// - `tx-txlock-conflict`: an input is already spent by an InstantSend-locked
///   transaction.
///
/// Neither settles the transaction (see the module docs); [`REFUSAL_QUORUM`]
/// of them send the probe to txid lookups.
///
/// `txn-mempool-conflict` is deliberately absent: another, unconfirmed and
/// not IS-locked, transaction spends the same input — nothing to look up.
const REFUSAL_REASONS: &[&str] = &[
    "bad-txns-inputs-missingorspent",
    "missing inputs",
    "tx-txlock-conflict",
];

/// Phrases of a gRPC status that a broken connection, not a node, produced.
const TRANSPORT_MARKERS: &[&str] = &[
    "h2 protocol error",
    "connection closed",
    "connection reset",
    "broken pipe",
    "transport error",
    // A gateway answered in plain HTTP (a 500 from a load balancer whose
    // upstream is down): no node's gRPC status.
    "grpc-status header missing",
];

/// What a single Core node said when handed a signed transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeVerdict {
    /// The node holds the transaction: it accepted it now, already had it in
    /// its mempool, or has it in a block (`-27`).
    Accepted,
    /// The node refused it for its inputs ([`REFUSAL_REASONS`]).
    Refused { reason: String },
    /// The node answered, but with no verdict: another rejection (such as
    /// `txn-mempool-conflict`).
    Unknown { reason: String },
    /// No node answered: unreachable, timed out, or no address left to try.
    Unreachable { reason: String },
}

/// Classify a failed `broadcastTransaction` gRPC response.
pub(crate) fn classify_failed_submission(code: Code, message: &str) -> NodeVerdict {
    // Transport-level: no node's answer reached us — including a stream the
    // connection broke mid-response, which tonic reports as Unknown or
    // Internal (Core's own rejections arrive with their reason instead).
    let lowered = message.to_ascii_lowercase();
    let broken_stream = matches!(code, Code::Unknown | Code::Internal)
        && TRANSPORT_MARKERS
            .iter()
            .any(|marker| lowered.contains(marker));
    // rs-dapi-client's own report that it could not open a channel to the
    // node (TLS, URI): the request never left.
    let never_opened = lowered.contains("channel creation failed");
    if broken_stream
        || never_opened
        || matches!(
            code,
            Code::Unavailable | Code::DeadlineExceeded | Code::Cancelled
        )
    {
        return NodeVerdict::Unreachable {
            reason: format!("{code:?}: {message}"),
        };
    }
    // `-27`: already in a block, by this node's word.
    if code == Code::AlreadyExists {
        return NodeVerdict::Accepted;
    }
    if REFUSAL_REASONS
        .iter()
        .any(|reason| lowered.contains(reason))
    {
        return NodeVerdict::Refused {
            reason: message.to_string(),
        };
    }
    NodeVerdict::Unknown {
        reason: format!("{code:?}: {message}"),
    }
}

/// The answer a probe gives about one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeVerdict {
    /// A node holds the transaction, in its mempool or in a block. Not
    /// settlement: it can still expire, lose to a conflict or sit in a block
    /// a reorg drops. The resolver keeps probing the send until the wallet
    /// settles it, which hands the nodes its bytes again; the verdict itself
    /// does not go back to Unresolved.
    Accepted,
    /// Not enough evidence that it landed; the transaction stays ambiguous.
    /// The reason says what the nodes answered, refusals included.
    Unresolved { reason: String },
}

/// Whether one node knows a txid (DAPI `getTransaction`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LookupAnswer {
    /// The node has the transaction, in its mempool or in a block.
    Known,
    /// The node answered and does not have it.
    NotFound,
    /// No answer: transport failure, timeout, anything else.
    Unknown { reason: String },
}

/// Talks to one Core node per call and reports which node answered and what
/// it said. A seam so the probe's evidence rules can be tested without a
/// network.
#[async_trait]
pub(crate) trait NodeSubmitter: Send + Sync {
    async fn submit(&self, transaction: &Transaction) -> (Option<String>, NodeVerdict);
    async fn lookup(&self, txid: &Txid) -> (Option<String>, LookupAnswer);
}

/// How many submissions one probe may spend looking for a verdict.
const MAX_SUBMISSIONS_PER_PROBE: usize = 4;

/// How many txid lookups one probe may spend after refusals.
const MAX_LOOKUPS_PER_PROBE: usize = 4;

/// How long a probe's request to one node may take, whatever timeout the SDK
/// it goes through is configured with (an SDK may run with none): set on the
/// request itself (see [`single_node`]), so a node that ran out is still
/// named. A probe is at most [`MAX_SUBMISSIONS_PER_PROBE`] plus
/// [`MAX_LOOKUPS_PER_PROBE`] requests, so this bounds how long one holds its
/// wallet's pass. A request that runs out is no answer.
const REQUEST_DEADLINE: Duration = Duration::from_secs(15);

/// The probe's own limit on one request, a little past [`REQUEST_DEADLINE`]:
/// it holds whatever the transport underneath does with its deadline.
const REQUEST_BACKSTOP: Duration = Duration::from_secs(20);

/// The reason of a request the backstop ended.
fn out_of_time() -> String {
    format!("no answer within {}s", REQUEST_BACKSTOP.as_secs())
}

/// How many *distinct* nodes must refuse a transaction for its inputs before
/// the probe looks the txid up. One refusal may be a node lagging behind the
/// tip; two make it worth asking whether it is a mined transaction whose
/// outputs are all spent.
const REFUSAL_QUORUM: usize = 2;

/// Resubmits a transaction whose broadcast outcome is unknown and turns the
/// nodes' answers into a verdict.
///
/// Resubmitting is safe: it is the same signed bytes, so a node either takes
/// a transaction it did not have — an extra delivery route — or tells us why
/// it will not. It can never create a second payment.
#[async_trait]
pub trait AcceptanceProbe: Send + Sync + 'static {
    /// What the nodes answered, judged within this one probe.
    async fn probe(&self, transaction: &Transaction) -> ProbeOutcome;
}

/// A probe's verdict, and whether any node answered at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeOutcome {
    pub verdict: ProbeVerdict,
    /// Some node answered a submission — with a refusal, or any rejection a
    /// node gives (a rate limit included). `false` when none did: no node
    /// reachable, timeouts, no address left to try — the probe learned
    /// nothing and is worth repeating soon. Not part of the verdict the host
    /// is told: it says how the probe went, not where the send stands.
    pub answered: bool,
}

impl ProbeOutcome {
    /// A verdict some node gave. An outcome no node answered is built
    /// explicitly (`answered: false`): never by default.
    pub fn answered(verdict: ProbeVerdict) -> Self {
        Self {
            verdict,
            answered: true,
        }
    }
}

/// Evidence rules shared by every probe: a node holding the transaction, in
/// its mempool or in a block, makes it accepted; [`REFUSAL_QUORUM`] distinct
/// nodes refusing it for its inputs send the probe to lookups, which find a
/// mined transaction whose outputs are all spent and otherwise leave it
/// unresolved; give up after
/// [`MAX_SUBMISSIONS_PER_PROBE`] submissions and [`MAX_LOOKUPS_PER_PROBE`]
/// lookups.
pub(crate) async fn probe_with(
    submitter: &dyn NodeSubmitter,
    transaction: &Transaction,
) -> ProbeOutcome {
    let mut refusing_nodes: HashSet<String> = HashSet::new();
    let mut first_refusal: Option<String> = None;
    let mut last_other = String::from("no submission was answered");
    let mut answered = false;

    for _ in 0..MAX_SUBMISSIONS_PER_PROBE {
        let (node, verdict) = timeout(REQUEST_BACKSTOP, submitter.submit(transaction))
            .await
            .unwrap_or_else(|_| {
                let reason = out_of_time();
                (None, NodeVerdict::Unreachable { reason })
            });
        answered |= !matches!(verdict, NodeVerdict::Unreachable { .. });
        tracing::debug!(
            txid = %transaction.txid(),
            node = node.as_deref().unwrap_or("unidentified"),
            ?verdict,
            "broadcast probe: node answered"
        );
        match verdict {
            NodeVerdict::Accepted => return ProbeOutcome::answered(ProbeVerdict::Accepted),
            NodeVerdict::Refused { reason } => {
                // An unidentified node cannot count towards the quorum — it
                // might be the node we already heard.
                if let Some(node) = node {
                    refusing_nodes.insert(node);
                }
                let first = first_refusal.get_or_insert(reason);
                if refusing_nodes.len() >= REFUSAL_QUORUM {
                    let reason = first.clone();
                    return ProbeOutcome::answered(
                        look_up_refused(submitter, transaction, reason).await,
                    );
                }
            }
            NodeVerdict::Unknown { reason } => last_other = reason,
            // A node's answer stays the reason over a later "no node
            // reached".
            NodeVerdict::Unreachable { reason } => {
                if !answered {
                    last_other = reason;
                }
            }
        }
    }

    let verdict = ProbeVerdict::Unresolved {
        reason: match first_refusal {
            None => last_other,
            Some(reason) => format!(
                "{} of {REFUSAL_QUORUM} nodes needed refused it as {reason}; last other answer: {last_other}",
                refusing_nodes.len()
            ),
        },
    };
    ProbeOutcome { verdict, answered }
}

/// [`REFUSAL_QUORUM`] nodes refused the transaction as `reason`: ask by
/// `getTransaction` whether the txid is known at all. A node that has it — a
/// mined transaction whose outputs are all spent is refused the same way —
/// makes it accepted; nothing better than unresolved comes of nodes not
/// knowing it.
async fn look_up_refused(
    submitter: &dyn NodeSubmitter,
    transaction: &Transaction,
    reason: String,
) -> ProbeVerdict {
    let txid = &transaction.txid();
    let mut not_found: HashSet<String> = HashSet::new();
    let mut last_unknown = String::from("no lookup was answered");
    for _ in 0..MAX_LOOKUPS_PER_PROBE {
        let (node, answer) = timeout(REQUEST_BACKSTOP, submitter.lookup(txid))
            .await
            .unwrap_or_else(|_| {
                let reason = out_of_time();
                (None, LookupAnswer::Unknown { reason })
            });
        tracing::debug!(
            txid = %txid,
            node = node.as_deref().unwrap_or("unidentified"),
            ?answer,
            "broadcast probe: txid lookup answered"
        );
        match answer {
            LookupAnswer::Known => return ProbeVerdict::Accepted,
            LookupAnswer::NotFound => {
                if let Some(node) = node {
                    not_found.insert(node);
                }
                // Enough nodes say they do not have it: asking on learns
                // nothing more.
                if not_found.len() >= REFUSAL_QUORUM {
                    break;
                }
            }
            LookupAnswer::Unknown { reason } => last_unknown = reason,
        }
    }
    ProbeVerdict::Unresolved {
        reason: format!(
            "refused as {reason}; {} node(s) do not know the txid; last other lookup answer: {last_unknown}",
            not_found.len()
        ),
    }
}

/// Which node answered a `broadcastTransaction` submission and what it said.
/// A failure that is not a node's gRPC status (no address left to try, a
/// connection that never opened) is no answer at all.
fn submit_reply(
    result: ExecutionResult<BroadcastTransactionResponse, DapiClientError>,
) -> (Option<String>, NodeVerdict) {
    match result {
        Ok(response) => (Some(response.address.to_string()), NodeVerdict::Accepted),
        Err(error) => {
            let node = error.address.as_ref().map(ToString::to_string);
            let verdict = match &error.inner {
                // tonic attaches the transport error (h2, hyper, io) as the
                // source of a status it made itself; a node's own status has
                // none. The message phrases are only a fallback.
                DapiClientError::Transport(TransportError::Grpc(status))
                    if matches!(status.code(), Code::Unknown | Code::Internal)
                        && std::error::Error::source(status).is_some() =>
                {
                    NodeVerdict::Unreachable {
                        reason: format!("{:?}: {}", status.code(), status.message()),
                    }
                }
                DapiClientError::Transport(TransportError::Grpc(status)) => {
                    classify_failed_submission(status.code(), status.message())
                }
                other => NodeVerdict::Unreachable {
                    reason: other.to_string(),
                },
            };
            (node, verdict)
        }
    }
}

/// Which node answered a `getTransaction` lookup and what the answer says. A
/// reply is read the way the SDK reads one ([`transaction_from_reply`]): an
/// empty one, or one carrying another transaction, is the node not showing the
/// txid, like gRPC `NOT_FOUND`; bytes that do not decode are no answer —
/// neither is evidence that it exists.
fn lookup_reply(
    txid: &Txid,
    result: ExecutionResult<GetTransactionResponse, DapiClientError>,
) -> (Option<String>, LookupAnswer) {
    match result {
        Ok(response) => {
            let node = Some(response.address.to_string());
            let answer = match transaction_from_reply(&response.inner, txid) {
                Ok(None) => LookupAnswer::NotFound,
                Ok(Some(_)) => lookup_answer(&response.inner),
                Err(error) => LookupAnswer::Unknown {
                    reason: format!("unusable reply: {error}"),
                },
            };
            (node, answer)
        }
        Err(error) => {
            let node = error.address.as_ref().map(ToString::to_string);
            let answer = match &error.inner {
                DapiClientError::Transport(TransportError::Grpc(status))
                    if status.code() == Code::NotFound =>
                {
                    LookupAnswer::NotFound
                }
                other => LookupAnswer::Unknown {
                    reason: other.to_string(),
                },
            };
            (node, answer)
        }
    }
}

/// What a node's `getTransaction` answer says about a transaction it holds.
/// A block hash with no height and
/// no confirmations is a block off the node's active chain (a stale fork; both
/// DAPI servers send height 0 for Core's -1): neither mined nor in a mempool.
fn lookup_answer(tx: &GetTransactionResponse) -> LookupAnswer {
    let in_block = tx.height > 0 || tx.confirmations > 0;
    if !in_block && !tx.block_hash.is_empty() {
        return LookupAnswer::Unknown {
            reason: "known only from a block off the active chain".to_string(),
        };
    }
    LookupAnswer::Known
}

/// One node per request: the probe decides whether to ask another, and it has
/// to know which node each answer came from. A probe never bans the address
/// it reached: it runs exactly when the network is flaky, and the SDK's
/// shared address list would carry its bans to every other caller — and to
/// the probe's own retries.
fn single_node() -> RequestSettings {
    RequestSettings {
        retries: Some(0),
        ban_failed_address: Some(false),
        timeout: Some(REQUEST_DEADLINE),
        ..RequestSettings::default()
    }
}

/// Talks to evonodes through DAPI: `broadcastTransaction` (a Core
/// `sendrawtransaction`) and `getTransaction`, a random node per request.
pub(crate) struct DapiNodeSubmitter {
    sdk: Arc<dash_sdk::Sdk>,
}

#[async_trait]
impl NodeSubmitter for DapiNodeSubmitter {
    async fn submit(&self, transaction: &Transaction) -> (Option<String>, NodeVerdict) {
        let request = BroadcastTransactionRequest {
            transaction: consensus::serialize(transaction),
            allow_high_fees: false,
            bypass_limits: false,
        };
        submit_reply(self.sdk.execute(request, single_node()).await)
    }

    async fn lookup(&self, txid: &Txid) -> (Option<String>, LookupAnswer) {
        // `id` is the display hex, as Core's getrawtransaction takes it.
        let request = GetTransactionRequest {
            id: txid.to_string(),
        };
        lookup_reply(txid, self.sdk.execute(request, single_node()).await)
    }
}

/// The production probe: resubmits through DAPI.
pub struct DapiAcceptanceProbe {
    submitter: DapiNodeSubmitter,
}

impl DapiAcceptanceProbe {
    pub fn new(sdk: Arc<dash_sdk::Sdk>) -> Self {
        Self {
            submitter: DapiNodeSubmitter { sdk },
        }
    }
}

#[async_trait]
impl AcceptanceProbe for DapiAcceptanceProbe {
    async fn probe(&self, transaction: &Transaction) -> ProbeOutcome {
        probe_with(&self.submitter, transaction).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use dash_sdk::dapi_client::{ExecutionError, ExecutionResponse};
    use dash_sdk::dapi_grpc::tonic::{Code, Status};

    use super::*;

    fn transaction() -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: Vec::new(),
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    /// Replays scripted node answers in order and counts calls. Lookups
    /// default to two distinct nodes not knowing the txid, unless a test
    /// scripts otherwise.
    struct ScriptedNodes {
        answers: Mutex<VecDeque<(Option<&'static str>, NodeVerdict)>>,
        lookups: Mutex<VecDeque<(Option<&'static str>, LookupAnswer)>>,
        submissions: Mutex<usize>,
        lookups_made: Mutex<usize>,
    }

    impl ScriptedNodes {
        fn new(answers: Vec<(Option<&'static str>, NodeVerdict)>) -> Self {
            Self::with_lookups(
                answers,
                vec![
                    (Some("x"), LookupAnswer::NotFound),
                    (Some("y"), LookupAnswer::NotFound),
                ],
            )
        }

        fn with_lookups(
            answers: Vec<(Option<&'static str>, NodeVerdict)>,
            lookups: Vec<(Option<&'static str>, LookupAnswer)>,
        ) -> Self {
            Self {
                answers: Mutex::new(answers.into()),
                lookups: Mutex::new(lookups.into()),
                submissions: Mutex::new(0),
                lookups_made: Mutex::new(0),
            }
        }

        fn submissions(&self) -> usize {
            *self.submissions.lock().expect("submissions")
        }

        fn lookups_made(&self) -> usize {
            *self.lookups_made.lock().expect("lookups")
        }
    }

    #[async_trait]
    impl NodeSubmitter for ScriptedNodes {
        async fn submit(&self, _transaction: &Transaction) -> (Option<String>, NodeVerdict) {
            *self.submissions.lock().expect("submissions") += 1;
            let (node, verdict) = self
                .answers
                .lock()
                .expect("answers")
                .pop_front()
                .unwrap_or((
                    None,
                    NodeVerdict::Unknown {
                        reason: "script exhausted".to_string(),
                    },
                ));
            (node.map(str::to_string), verdict)
        }

        async fn lookup(&self, _txid: &Txid) -> (Option<String>, LookupAnswer) {
            *self.lookups_made.lock().expect("lookups") += 1;
            let (node, answer) = self
                .lookups
                .lock()
                .expect("lookups")
                .pop_front()
                .unwrap_or((
                    None,
                    LookupAnswer::Unknown {
                        reason: "script exhausted".to_string(),
                    },
                ));
            (node.map(str::to_string), answer)
        }
    }

    fn known() -> LookupAnswer {
        LookupAnswer::Known
    }

    fn conflict() -> NodeVerdict {
        NodeVerdict::Refused {
            reason: "Transaction is rejected: tx-txlock-conflict".to_string(),
        }
    }

    fn refused() -> NodeVerdict {
        NodeVerdict::Refused {
            reason: "bad-txns-inputs-missingorspent".to_string(),
        }
    }

    fn unknown() -> NodeVerdict {
        NodeVerdict::Unknown {
            reason: "Unavailable: connection refused".to_string(),
        }
    }

    fn is_unresolved(verdict: &ProbeVerdict) -> bool {
        matches!(verdict, ProbeVerdict::Unresolved { .. })
    }

    #[test]
    fn should_classify_core_answers_the_way_both_dapi_servers_surface_them() {
        let cases = [
            // -27, both servers.
            (
                Code::AlreadyExists,
                "Transaction already in chain: Transaction already in block chain",
                NodeVerdict::Accepted,
            ),
            // -25, JS dapi.
            (
                Code::InvalidArgument,
                "invalid transaction: bad-txns-inputs-missingorspent",
                NodeVerdict::Refused {
                    reason: "invalid transaction: bad-txns-inputs-missingorspent".to_string(),
                },
            ),
            // -25, rs-dapi.
            (
                Code::FailedPrecondition,
                "Transaction is rejected: bad-txns-inputs-missingorspent",
                NodeVerdict::Refused {
                    reason: "Transaction is rejected: bad-txns-inputs-missingorspent".to_string(),
                },
            ),
            // -26 against an IS-locked spend.
            (
                Code::FailedPrecondition,
                "Transaction is rejected: tx-txlock-conflict",
                conflict(),
            ),
            // -26 against an unconfirmed spend.
            (
                Code::FailedPrecondition,
                "Transaction is rejected: txn-mempool-conflict",
                NodeVerdict::Unknown {
                    reason: "FailedPrecondition: Transaction is rejected: txn-mempool-conflict"
                        .to_string(),
                },
            ),
        ];

        for (code, message, expected) in cases {
            assert_eq!(
                classify_failed_submission(code, message),
                expected,
                "{code:?} {message}"
            );
        }
    }

    /// Neither a transport failure nor an unknown rejection is a verdict;
    /// only the second is a node's answer.
    #[test]
    fn should_leave_transport_failures_and_unknown_rejections_without_a_verdict() {
        for (code, message) in [
            (Code::Unavailable, "connection refused"),
            (Code::DeadlineExceeded, "timeout"),
        ] {
            assert!(
                matches!(
                    classify_failed_submission(code, message),
                    NodeVerdict::Unreachable { .. }
                ),
                "{code:?} {message}"
            );
        }
        for (code, message) in [
            (
                Code::InvalidArgument,
                "invalid transaction: TX decode failed",
            ),
            (Code::Internal, "bad-txns-in-belowout"),
            // A gateway's rate limit is a node's answer: back off, not an
            // outage.
            (Code::ResourceExhausted, "rate limited"),
        ] {
            assert!(
                matches!(
                    classify_failed_submission(code, message),
                    NodeVerdict::Unknown { .. }
                ),
                "{code:?} {message}"
            );
        }
    }

    #[tokio::test]
    async fn should_accept_as_soon_as_any_node_holds_the_transaction() {
        let nodes = ScriptedNodes::new(vec![
            (Some("a"), refused()),
            (Some("b"), NodeVerdict::Accepted),
        ]);

        assert_eq!(
            probe_with(&nodes, &transaction()).await.verdict,
            ProbeVerdict::Accepted
        );
        assert_eq!(nodes.submissions(), 2);
    }

    /// Refusals never settle a transaction: a lagging node, an ancestor
    /// still propagating and a reorgable competing spend all look alike.
    #[tokio::test]
    async fn should_never_settle_a_transaction_on_refusals_and_unknown_txids() {
        for answers in [
            vec![(Some("a"), refused()), (Some("b"), refused())],
            vec![(Some("a"), conflict()), (Some("b"), conflict())],
            vec![(Some("a"), conflict()), (Some("b"), refused())],
        ] {
            let nodes = ScriptedNodes::new(answers);
            let verdict = probe_with(&nodes, &transaction()).await.verdict;

            assert!(is_unresolved(&verdict), "{verdict:?}");
            assert_eq!(nodes.lookups_made(), REFUSAL_QUORUM);
        }
    }

    #[tokio::test]
    async fn should_keep_the_refusal_in_the_unresolved_reason() {
        let nodes = ScriptedNodes::new(vec![(Some("a"), conflict()), (Some("b"), conflict())]);

        let ProbeVerdict::Unresolved { reason } = probe_with(&nodes, &transaction()).await.verdict
        else {
            panic!("expected unresolved");
        };
        assert!(reason.contains("tx-txlock-conflict"), "{reason}");
    }

    /// A lagging node answers `missingorspent` for a valid transaction. Hearing
    /// the same node twice must not look like two opinions.
    #[tokio::test]
    async fn should_not_count_the_same_node_twice_towards_the_refusal_quorum() {
        let nodes = ScriptedNodes::new(vec![
            (Some("a"), refused()),
            (Some("a"), refused()),
            (Some("a"), refused()),
            (Some("a"), refused()),
        ]);

        assert!(is_unresolved(
            &probe_with(&nodes, &transaction()).await.verdict
        ));
        assert_eq!(nodes.submissions(), MAX_SUBMISSIONS_PER_PROBE);
        assert_eq!(nodes.lookups_made(), 0);
    }

    #[tokio::test]
    async fn should_not_count_an_unidentified_node_towards_the_refusal_quorum() {
        let nodes = ScriptedNodes::new(vec![(Some("a"), refused()), (None, refused())]);

        assert!(is_unresolved(
            &probe_with(&nodes, &transaction()).await.verdict
        ));
        assert_eq!(nodes.lookups_made(), 0);
    }

    #[tokio::test]
    async fn should_stay_unresolved_when_nodes_only_fail() {
        let nodes = ScriptedNodes::new(vec![
            (
                Some("a"),
                NodeVerdict::Unknown {
                    reason: "txn-mempool-conflict".to_string(),
                },
            ),
            (Some("b"), unknown()),
            (Some("c"), unknown()),
            (Some("d"), unknown()),
        ]);

        assert!(is_unresolved(
            &probe_with(&nodes, &transaction()).await.verdict
        ));
        assert_eq!(nodes.submissions(), MAX_SUBMISSIONS_PER_PROBE);
    }

    #[tokio::test]
    async fn should_reach_the_refusal_quorum_across_intervening_failures() {
        let nodes = ScriptedNodes::new(vec![
            (Some("a"), refused()),
            (Some("b"), unknown()),
            (Some("c"), refused()),
        ]);

        assert!(is_unresolved(
            &probe_with(&nodes, &transaction()).await.verdict
        ));
        assert_eq!(nodes.submissions(), 3);
        assert_eq!(nodes.lookups_made(), REFUSAL_QUORUM);
    }

    /// Core answers `missingorspent` for a mined transaction whose outputs
    /// are all spent (testnet `e758f06a…`, 2026-09-29), and for one still in
    /// a mempool that lost a double-spend race on the refusing nodes. A node
    /// that knows the txid turns the refusals into Accepted, at the first
    /// lookup.
    #[tokio::test]
    async fn should_accept_a_refused_transaction_a_node_still_knows() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), refused()), (Some("b"), refused())],
            vec![(Some("c"), known())],
        );

        assert_eq!(
            probe_with(&nodes, &transaction()).await.verdict,
            ProbeVerdict::Accepted
        );
        assert_eq!(nodes.lookups_made(), 1);
    }

    /// A node's "already in a block" (`-27`) is acceptance, with no lookup
    /// for a second opinion: the probe does not tell a block from a mempool.
    #[tokio::test]
    async fn should_accept_on_one_nodes_word_that_it_is_in_a_block() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(
                Some("a"),
                classify_failed_submission(Code::AlreadyExists, "Transaction already in chain"),
            )],
            vec![(Some("b"), known())],
        );

        assert_eq!(
            probe_with(&nodes, &transaction()).await.verdict,
            ProbeVerdict::Accepted
        );
        assert_eq!(nodes.submissions(), 1);
        assert_eq!(nodes.lookups_made(), 0);
    }

    #[test]
    fn should_read_a_stale_fork_lookup_as_neither_mined_nor_mempool() {
        let answer = |block_hash: Vec<u8>, height, confirmations| {
            lookup_answer(&GetTransactionResponse {
                block_hash,
                height,
                confirmations,
                ..GetTransactionResponse::default()
            })
        };

        assert_eq!(answer(Vec::new(), 0, 0), known());
        assert_eq!(answer(vec![1; 32], 100, 3), known());
        assert!(matches!(
            answer(vec![1; 32], 0, 0),
            LookupAnswer::Unknown { .. }
        ));
    }

    /// The production lookup adapter reads a reply as the SDK does: empty
    /// bytes or another transaction are the node not showing the txid,
    /// malformed bytes are no answer — neither is evidence that it exists —
    /// and the node that answered is kept either way.
    #[test]
    fn should_not_take_an_empty_or_malformed_lookup_reply_as_known() {
        let held = Transaction {
            lock_time: 7,
            ..transaction()
        };
        let reply = |bytes: Vec<u8>| {
            Ok(ExecutionResponse {
                inner: GetTransactionResponse {
                    transaction: bytes,
                    ..GetTransactionResponse::default()
                },
                retries: 0,
                address: "http://127.0.0.1:1443".parse().expect("address"),
            })
        };
        let txid = held.txid();

        let (node, empty) = lookup_reply(&txid, reply(Vec::new()));
        assert_eq!(empty, LookupAnswer::NotFound);
        assert!(node.is_some(), "the serving node is kept");

        let (node, malformed) = lookup_reply(&txid, reply(vec![0xff, 0x01]));
        assert!(
            matches!(malformed, LookupAnswer::Unknown { .. }),
            "{malformed:?}"
        );
        assert!(node.is_some());

        let other = consensus::serialize(&transaction());
        let (_, mismatched) = lookup_reply(&txid, reply(other));
        assert_eq!(mismatched, LookupAnswer::NotFound);

        let (_, answer) = lookup_reply(&txid, reply(consensus::serialize(&held)));
        assert_eq!(answer, known());
    }

    /// A node's answer stays the reason when later submissions reach no
    /// node: the conflict is what the log should show, not the outage.
    #[tokio::test]
    async fn should_keep_a_nodes_answer_as_the_reason_over_later_unreachable_ones() {
        let unreachable = || NodeVerdict::Unreachable {
            reason: "no available addresses".to_string(),
        };
        let nodes = ScriptedNodes::new(vec![
            (
                Some("a"),
                NodeVerdict::Unknown {
                    reason: "txn-mempool-conflict".to_string(),
                },
            ),
            (None, unreachable()),
            (None, unreachable()),
            (None, unreachable()),
        ]);

        let outcome = probe_with(&nodes, &transaction()).await;

        assert!(outcome.answered);
        let ProbeVerdict::Unresolved { reason } = outcome.verdict else {
            panic!("unresolved");
        };
        assert!(reason.contains("txn-mempool-conflict"), "{reason}");
    }

    /// A rate-limited submission is a node's answer: no fast retry.
    #[tokio::test]
    async fn should_count_a_rate_limit_as_an_answer() {
        let limited = || NodeVerdict::Unknown {
            reason: "ResourceExhausted: rate limited".to_string(),
        };
        let nodes = ScriptedNodes::new(vec![
            (Some("a"), limited()),
            (Some("b"), limited()),
            (Some("c"), limited()),
            (Some("d"), limited()),
        ]);
        assert!(probe_with(&nodes, &transaction()).await.answered);
        assert!(matches!(
            classify_failed_submission(Code::ResourceExhausted, "rate limited"),
            NodeVerdict::Unknown { .. }
        ));
    }

    /// The live log's case: every address banned, so the SDK fails with
    /// `NoAvailableAddresses` before any node is asked — no answer, which
    /// arms the resolver's retry. A node's status is an answer.
    #[test]
    fn should_read_a_submission_no_node_received_as_unreachable() {
        let failed = |inner: DapiClientError| {
            Err(ExecutionError {
                inner,
                retries: 0,
                address: None,
            })
        };
        let (node, verdict) = submit_reply(failed(DapiClientError::NoAvailableAddresses));
        assert!(node.is_none());
        assert!(
            matches!(verdict, NodeVerdict::Unreachable { .. }),
            "{verdict:?}"
        );

        let (_, verdict) = submit_reply(failed(DapiClientError::Transport(TransportError::Grpc(
            Status::failed_precondition("txn-mempool-conflict"),
        ))));
        assert!(
            matches!(verdict, NodeVerdict::Unknown { .. }),
            "{verdict:?}"
        );

        // A stream that broke mid-response, whatever its wording: tonic's
        // status carries the transport error as its source.
        let broken = Status::from_error(Box::new(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected end of file",
        )));
        let (_, verdict) = submit_reply(failed(DapiClientError::Transport(TransportError::Grpc(
            broken,
        ))));
        assert!(
            matches!(verdict, NodeVerdict::Unreachable { .. }),
            "{verdict:?}"
        );
        // A node's Unknown/Internal with no transport source is an answer.
        let (_, verdict) = submit_reply(failed(DapiClientError::Transport(TransportError::Grpc(
            Status::internal("bad-txns-in-belowout"),
        ))));
        assert!(
            matches!(verdict, NodeVerdict::Unknown { .. }),
            "{verdict:?}"
        );
        let (_, verdict) = submit_reply(failed(DapiClientError::Transport(TransportError::Grpc(
            Status::cancelled("cancelled"),
        ))));
        assert!(
            matches!(verdict, NodeVerdict::Unreachable { .. }),
            "{verdict:?}"
        );
    }

    /// A probe never bans the address it reached, and asks one node per
    /// request.
    #[test]
    fn should_probe_without_banning_and_without_retries() {
        let settings = single_node();
        assert_eq!(settings.ban_failed_address, Some(false));
        assert_eq!(settings.retries, Some(0));
    }

    /// A failed lookup: gRPC `NOT_FOUND` is the node not knowing the txid,
    /// anything else no answer — the node, when known, is kept either way
    /// (the refusal quorum counts distinct nodes).
    #[test]
    fn should_read_a_failed_lookup_by_its_status_and_keep_the_node() {
        let failed = |status: Status| {
            Err(ExecutionError {
                inner: DapiClientError::Transport(TransportError::Grpc(status)),
                retries: 0,
                address: Some("http://127.0.0.1:1443".parse().expect("address")),
            })
        };
        let txid = transaction().txid();

        let (node, missing) = lookup_reply(&txid, failed(Status::not_found("no such tx")));
        assert_eq!(missing, LookupAnswer::NotFound);
        assert!(node.is_some(), "the node that answered is kept");

        let (node, other) = lookup_reply(&txid, failed(Status::unavailable("down")));
        assert!(matches!(other, LookupAnswer::Unknown { .. }), "{other:?}");
        assert!(node.is_some());
    }

    #[tokio::test]
    async fn should_stop_looking_up_once_two_distinct_nodes_do_not_know_the_txid() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), refused()), (Some("b"), refused())],
            vec![
                (Some("c"), LookupAnswer::NotFound),
                (Some("c"), LookupAnswer::NotFound),
                (None, LookupAnswer::NotFound),
                (Some("d"), LookupAnswer::NotFound),
            ],
        );

        assert!(is_unresolved(
            &probe_with(&nodes, &transaction()).await.verdict
        ));
        assert_eq!(nodes.lookups_made(), MAX_LOOKUPS_PER_PROBE);
    }

    /// A probe tells whether any node answered: every submission failing
    /// without an answer (no reachable node) is `answered: false` — worth
    /// repeating soon — while a refusal short of the quorum, or any other
    /// rejection, is an answer.
    #[tokio::test]
    async fn should_tell_a_probe_no_node_answered_from_one_a_node_refused() {
        let unreachable = || NodeVerdict::Unreachable {
            reason: "no available addresses".to_string(),
        };
        let nodes = ScriptedNodes::new(vec![
            (None, unreachable()),
            (None, unreachable()),
            (None, unreachable()),
            (None, unreachable()),
        ]);
        assert!(!probe_with(&nodes, &transaction()).await.answered);

        for answer in [
            refused(),
            NodeVerdict::Unknown {
                reason: "txn-mempool-conflict".to_string(),
            },
        ] {
            let nodes = ScriptedNodes::new(vec![
                (Some("a"), answer),
                (None, unreachable()),
                (None, unreachable()),
                (None, unreachable()),
            ]);
            assert!(probe_with(&nodes, &transaction()).await.answered);
        }
        for (code, message) in [
            (Code::DeadlineExceeded, "no complete response"),
            (
                Code::Internal,
                "h2 protocol error: error reading a body from connection",
            ),
            (
                Code::Unknown,
                "grpc-status header missing, mapped from HTTP status code 500",
            ),
            (
                Code::InvalidArgument,
                "Channel creation failed: invalid uri",
            ),
            (Code::Unknown, "connection closed before message completed"),
        ] {
            assert!(
                matches!(
                    classify_failed_submission(code, message),
                    NodeVerdict::Unreachable { .. }
                ),
                "{code:?} {message}"
            );
        }
    }

    #[tokio::test]
    async fn should_not_look_up_a_transaction_nobody_refused() {
        let nodes = ScriptedNodes::new(vec![(Some("a"), NodeVerdict::Accepted)]);

        probe_with(&nodes, &transaction()).await;

        assert_eq!(nodes.lookups_made(), 0);
    }

    /// Nodes that never answer, as behind an SDK configured with no request
    /// timeout.
    struct SilentNodes;

    #[async_trait]
    impl NodeSubmitter for SilentNodes {
        async fn submit(&self, _transaction: &Transaction) -> (Option<String>, NodeVerdict) {
            std::future::pending().await
        }

        async fn lookup(&self, _txid: &Txid) -> (Option<String>, LookupAnswer) {
            std::future::pending().await
        }
    }

    /// A probe ends by its own deadline when no node ever answers: it does
    /// not hold its wallet's pass for as long as the transport waits.
    #[tokio::test(start_paused = true)]
    async fn should_end_a_probe_whose_requests_are_never_answered() {
        let started = tokio::time::Instant::now();

        let outcome = probe_with(&SilentNodes, &transaction()).await;

        assert!(!outcome.answered);
        assert!(is_unresolved(&outcome.verdict));
        assert_eq!(
            started.elapsed(),
            REQUEST_BACKSTOP * MAX_SUBMISSIONS_PER_PROBE as u32
        );
    }

    /// A lookup that is never answered does not take back the refusals the
    /// probe already heard: it ends unresolved, as an answered probe.
    #[tokio::test(start_paused = true)]
    async fn should_end_a_lookup_that_is_never_answered() {
        struct RefusingThenSilent(ScriptedNodes);

        #[async_trait]
        impl NodeSubmitter for RefusingThenSilent {
            async fn submit(&self, transaction: &Transaction) -> (Option<String>, NodeVerdict) {
                self.0.submit(transaction).await
            }

            async fn lookup(&self, _txid: &Txid) -> (Option<String>, LookupAnswer) {
                std::future::pending().await
            }
        }
        let nodes = RefusingThenSilent(ScriptedNodes::new(vec![
            (Some("a"), refused()),
            (Some("b"), refused()),
        ]));

        let outcome = probe_with(&nodes, &transaction()).await;

        assert!(outcome.answered);
        assert!(is_unresolved(&outcome.verdict));
    }
}
