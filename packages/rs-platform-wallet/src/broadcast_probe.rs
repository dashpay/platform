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
//! The probe uses the positive answers: a node holding the transaction makes
//! it accepted, and nodes having it in a block make it mined. A refusal never
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
//! Mined needs two distinct nodes reporting the transaction in a block
//! ([`MINED_QUORUM`]); one such report makes it accepted — it exists — and the
//! second opinion is asked by lookup, not by resubmitting. On a network with
//! a single reachable evonode (a devnet pinned to one address) nothing is
//! ever mined by probe, only accepted; the wallet's own block processing
//! settles it. Mined is the nodes' word, not settlement: the resolver drops it
//! and asks again if the wallet, following the tip, has not seen the
//! transaction a few blocks later (a block a reorg dropped, or one the chain
//! the wallet follows never had).
//!
//! The two DAPI implementations surface those codes differently — the JS
//! server maps `-25` to `InvalidArgument`, rs-dapi to `FailedPrecondition` —
//! so the classification keys on Core's reason text, which both pass through
//! in the status message, and on `AlreadyExists` for `-27`, which both agree
//! on.
//!
//! [`BroadcastError::MaybeSent`]: crate::broadcaster::BroadcastError::MaybeSent

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use dash_sdk::dapi_client::transport::TransportError;
use dash_sdk::dapi_client::{DapiClientError, DapiRequestExecutor, RequestSettings};
use dash_sdk::dapi_grpc::core::v0::{
    BroadcastTransactionRequest, GetTransactionRequest, GetTransactionResponse,
};
use dash_sdk::dapi_grpc::tonic::Code;
use dashcore::consensus;
use dashcore::{Transaction, Txid};

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

/// What a single Core node said when handed a signed transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeVerdict {
    /// The node holds the transaction in its mempool — it accepted it now or
    /// already had it.
    Accepted,
    /// The node has the transaction in a block (`-27`).
    Mined,
    /// The node refused it for its inputs ([`REFUSAL_REASONS`]).
    Refused { reason: String },
    /// No verdict: transport failure, timeout, or another rejection (such as
    /// `txn-mempool-conflict`).
    Unknown { reason: String },
}

/// Classify a failed `broadcastTransaction` gRPC response.
pub(crate) fn classify_failed_submission(code: Code, message: &str) -> NodeVerdict {
    if code == Code::AlreadyExists {
        return NodeVerdict::Mined;
    }
    let lowered = message.to_ascii_lowercase();
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
    /// A node holds the transaction in its mempool, or one node — short of
    /// [`MINED_QUORUM`] — has it in a block. Not settlement: it can still
    /// expire or lose to a conflict, so the resolver keeps asking.
    Accepted,
    /// Two different nodes have the transaction in a block. The resolver stops
    /// asking — unless the wallet, following the tip, has not seen it a few
    /// blocks later; then the verdict is cleared and it asks again.
    Mined,
    /// Not enough evidence that it landed; the transaction stays ambiguous.
    /// The reason says what the nodes answered, refusals included.
    Unresolved { reason: String },
}

/// Whether one node knows a txid (DAPI `getTransaction`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LookupAnswer {
    /// The node has the transaction; `mined` when it is in a block, at
    /// `height` when the node named one.
    Known { mined: bool, height: Option<u32> },
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

/// How many txid lookups one probe may spend: after refusals, or to get a
/// second opinion on one node's "in a block".
const MAX_LOOKUPS_PER_PROBE: usize = 4;

/// How many *distinct* nodes must refuse a transaction for its inputs before
/// the probe looks the txid up. One refusal may be a node lagging behind the
/// tip; two make it worth asking whether it is a mined transaction whose
/// outputs are all spent.
const REFUSAL_QUORUM: usize = 2;

/// How many *distinct* nodes must report a transaction in a block before it
/// counts as mined — final, never asked about again, its children probed in
/// its place. One node on a stale fork, or one whose block is about to be
/// reorged out, would otherwise settle it on false evidence. A single such
/// answer still shows the transaction exists: it counts as accepted.
pub(crate) const MINED_QUORUM: usize = 2;

/// Resubmits a transaction whose broadcast outcome is unknown and turns the
/// nodes' answers into a verdict.
///
/// Resubmitting is safe: it is the same signed bytes, so a node either takes
/// a transaction it did not have — an extra delivery route — or tells us why
/// it will not. It can never create a second payment.
#[async_trait]
pub trait AcceptanceProbe: Send + Sync + 'static {
    /// The verdict, and what else the probe learned.
    async fn probe(&self, transaction: &Transaction) -> ProbeReport;
}

/// A probe's verdict, and the identified nodes that reported the transaction
/// in a block. The resolver adds these up across probes of one root:
/// [`MINED_QUORUM`] distinct nodes over several probes make it mined too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub verdict: ProbeVerdict,
    pub in_block_by: BTreeSet<String>,
    /// The highest block height a node's lookup placed the transaction at,
    /// when one did (a submission's "already in the block chain" names none).
    pub block_height: Option<u32>,
}

impl From<ProbeVerdict> for ProbeReport {
    fn from(verdict: ProbeVerdict) -> Self {
        Self {
            verdict,
            in_block_by: BTreeSet::new(),
            block_height: None,
        }
    }
}

/// Evidence rules shared by every probe: a node holding the transaction in its
/// mempool makes it accepted; one reporting it in a block sends the probe to
/// lookups for a second opinion ([`MINED_QUORUM`] distinct nodes make it
/// mined); [`REFUSAL_QUORUM`] distinct nodes refusing it for its inputs send
/// the probe to lookups too, which find a mined transaction whose outputs are
/// all spent and otherwise leave it unresolved; give up after
/// [`MAX_SUBMISSIONS_PER_PROBE`] submissions and [`MAX_LOOKUPS_PER_PROBE`]
/// lookups.
pub(crate) async fn probe_with(
    submitter: &dyn NodeSubmitter,
    transaction: &Transaction,
) -> ProbeReport {
    let mut refusing_nodes: HashSet<String> = HashSet::new();
    let mut first_refusal: Option<String> = None;
    let mut last_other = String::from("no submission was answered");
    let report = ProbeReport::from;

    for _ in 0..MAX_SUBMISSIONS_PER_PROBE {
        let (node, verdict) = submitter.submit(transaction).await;
        tracing::debug!(
            txid = %transaction.txid(),
            node = node.as_deref().unwrap_or("unidentified"),
            ?verdict,
            "broadcast probe: node answered"
        );
        match verdict {
            NodeVerdict::Accepted => return report(ProbeVerdict::Accepted),
            NodeVerdict::Mined => {
                // In a block by this node's word. A second opinion comes from
                // a lookup, not from sending it again.
                return confirm_by_lookup(
                    submitter,
                    transaction,
                    LookupFor::SecondOpinion { seen_by: node },
                )
                .await;
            }
            NodeVerdict::Refused { reason } => {
                // An unidentified node cannot count towards the quorum — it
                // might be the node we already heard.
                if let Some(node) = node {
                    refusing_nodes.insert(node);
                }
                let first = first_refusal.get_or_insert(reason);
                if refusing_nodes.len() >= REFUSAL_QUORUM {
                    let reason = first.clone();
                    return confirm_by_lookup(
                        submitter,
                        transaction,
                        LookupFor::Refused { reason },
                    )
                    .await;
                }
            }
            NodeVerdict::Unknown { reason } => {
                last_other = reason;
            }
        }
    }

    report(ProbeVerdict::Unresolved {
        reason: match first_refusal {
            None => last_other,
            Some(reason) => format!(
                "{} of {REFUSAL_QUORUM} nodes needed refused it as {reason}; last other answer: {last_other}",
                refusing_nodes.len()
            ),
        },
    })
}

/// Why a probe turns to lookups.
enum LookupFor {
    /// A submission answered "already in a block" (from `seen_by`): a second
    /// node is asked.
    SecondOpinion { seen_by: Option<String> },
    /// [`REFUSAL_QUORUM`] nodes refused it as `reason`: is it a mined
    /// transaction whose outputs are all spent?
    Refused { reason: String },
}

/// Nodes that reported the transaction in a block, by submission or lookup.
#[derive(Default)]
struct MinedEvidence {
    nodes: HashSet<String>,
    /// The highest block height a lookup named.
    height: Option<u32>,
    /// Any node said so, identified or not: the transaction exists.
    seen: bool,
}

impl MinedEvidence {
    /// Count one report; whether the quorum is reached. An unidentified node
    /// cannot count — it might be one already heard.
    fn add(&mut self, node: Option<String>) -> bool {
        self.seen = true;
        if let Some(node) = node {
            self.nodes.insert(node);
        }
        self.nodes.len() >= MINED_QUORUM
    }
}

/// Settle by `getTransaction` lookups. A node that knows the txid in its
/// mempool makes it accepted; in a block, counts towards [`MINED_QUORUM`].
/// After refusals, nothing better than unresolved comes of nodes not knowing
/// it.
async fn confirm_by_lookup(
    submitter: &dyn NodeSubmitter,
    transaction: &Transaction,
    purpose: LookupFor,
) -> ProbeReport {
    let txid = &transaction.txid();
    let mut mined = MinedEvidence::default();
    let refused = match purpose {
        LookupFor::SecondOpinion { seen_by } => {
            mined.add(seen_by);
            None
        }
        LookupFor::Refused { reason } => Some(reason),
    };
    let report = |verdict, mined: &MinedEvidence| ProbeReport {
        verdict,
        in_block_by: mined.nodes.iter().cloned().collect(),
        block_height: mined.height,
    };
    let mut not_found: HashSet<String> = HashSet::new();
    let mut last_unknown = String::from("no lookup was answered");
    for _ in 0..MAX_LOOKUPS_PER_PROBE {
        let (node, answer) = submitter.lookup(txid).await;
        tracing::debug!(
            txid = %txid,
            node = node.as_deref().unwrap_or("unidentified"),
            ?answer,
            "broadcast probe: txid lookup answered"
        );
        match answer {
            LookupAnswer::Known {
                mined: true,
                height,
            } => {
                mined.height = mined.height.max(height);
                if mined.add(node) {
                    return report(ProbeVerdict::Mined, &mined);
                }
            }
            LookupAnswer::Known { mined: false, .. } => {
                return report(ProbeVerdict::Accepted, &mined)
            }
            LookupAnswer::NotFound => {
                if let Some(node) = node {
                    not_found.insert(node);
                }
                // Nobody has it in a block and enough nodes say so: asking on
                // learns nothing more.
                if refused.is_some() && !mined.seen && not_found.len() >= REFUSAL_QUORUM {
                    break;
                }
            }
            LookupAnswer::Unknown { reason } => last_unknown = reason,
        }
    }
    // A node's word that it is in a block: it exists, not settled as final.
    let Some(reason) = refused.filter(|_| !mined.seen) else {
        return report(ProbeVerdict::Accepted, &mined);
    };
    report(
        ProbeVerdict::Unresolved {
            reason: format!(
                "refused as {reason}; {} node(s) do not know the txid; last other lookup answer: {last_unknown}",
                not_found.len()
            ),
        },
        &mined,
    )
}

/// What a node's `getTransaction` answer says. A block hash with no height and
/// no confirmations is a block off the node's active chain (a stale fork; both
/// DAPI servers send height 0 for Core's -1): neither mined nor in a mempool.
fn lookup_answer(tx: &GetTransactionResponse) -> LookupAnswer {
    let mined = tx.height > 0 || tx.confirmations > 0;
    if !mined && !tx.block_hash.is_empty() {
        return LookupAnswer::Unknown {
            reason: "known only from a block off the active chain".to_string(),
        };
    }
    LookupAnswer::Known {
        mined,
        height: u32::try_from(tx.height).ok().filter(|height| *height > 0),
    }
}

/// One node per request: the probe decides whether to ask another, and it has
/// to know which node each answer came from.
fn single_node() -> RequestSettings {
    RequestSettings {
        retries: Some(0),
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
        match self.sdk.execute(request, single_node()).await {
            Ok(response) => (Some(response.address.to_string()), NodeVerdict::Accepted),
            Err(error) => {
                let node = error.address.as_ref().map(ToString::to_string);
                let verdict = match &error.inner {
                    DapiClientError::Transport(TransportError::Grpc(status)) => {
                        classify_failed_submission(status.code(), status.message())
                    }
                    other => NodeVerdict::Unknown {
                        reason: other.to_string(),
                    },
                };
                (node, verdict)
            }
        }
    }

    async fn lookup(&self, txid: &Txid) -> (Option<String>, LookupAnswer) {
        // `id` is the display hex, as Core's getrawtransaction takes it.
        let request = GetTransactionRequest {
            id: txid.to_string(),
        };
        match self.sdk.execute(request, single_node()).await {
            Ok(response) => (
                Some(response.address.to_string()),
                lookup_answer(&response.inner),
            ),
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
    async fn probe(&self, transaction: &Transaction) -> ProbeReport {
        probe_with(&self.submitter, transaction).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use dash_sdk::dapi_grpc::tonic::Code;

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

    fn in_block() -> LookupAnswer {
        LookupAnswer::Known {
            mined: true,
            height: None,
        }
    }

    fn in_mempool() -> LookupAnswer {
        LookupAnswer::Known {
            mined: false,
            height: None,
        }
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
                NodeVerdict::Mined,
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

    #[test]
    fn should_leave_transport_failures_and_unknown_rejections_without_a_verdict() {
        for (code, message) in [
            (Code::Unavailable, "connection refused"),
            (Code::DeadlineExceeded, "timeout"),
            (
                Code::InvalidArgument,
                "invalid transaction: TX decode failed",
            ),
            (Code::Internal, "bad-txns-in-belowout"),
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
    /// are all spent (testnet `e758f06a…`, 2026-09-29). Nodes that still know
    /// the txid in a block must turn the refusals into Mined.
    #[tokio::test]
    async fn should_report_mined_for_a_refused_transaction_nodes_have_in_a_block() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), refused()), (Some("b"), refused())],
            vec![(Some("c"), in_block()), (Some("d"), in_block())],
        );

        assert_eq!(
            probe_with(&nodes, &transaction()).await.verdict,
            ProbeVerdict::Mined
        );
        assert_eq!(nodes.lookups_made(), 2);
    }

    /// One node on a stale fork (or with a block about to be reorged out)
    /// must not settle a transaction as mined: two distinct nodes must.
    #[tokio::test]
    async fn should_report_mined_only_when_two_distinct_nodes_have_it_in_a_block() {
        let two = ScriptedNodes::with_lookups(
            vec![(Some("a"), NodeVerdict::Mined)],
            vec![(Some("b"), in_block())],
        );
        let same_twice = ScriptedNodes::with_lookups(
            vec![(Some("a"), NodeVerdict::Mined)],
            vec![(Some("a"), in_block())],
        );

        assert_eq!(
            probe_with(&two, &transaction()).await.verdict,
            ProbeVerdict::Mined
        );
        assert_eq!(two.submissions(), 1, "the second opinion is a lookup");
        assert_eq!(
            probe_with(&same_twice, &transaction()).await.verdict,
            ProbeVerdict::Accepted,
            "one node's word: it exists, not that it is final"
        );
    }

    /// The block height a lookup names rides on the report, for the resolver
    /// to count its grace from.
    #[tokio::test]
    async fn should_report_the_block_height_a_lookup_named() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), NodeVerdict::Mined)],
            vec![(
                Some("b"),
                LookupAnswer::Known {
                    mined: true,
                    height: Some(1_234),
                },
            )],
        );
        let report = probe_with(&nodes, &transaction()).await;
        assert_eq!(report.verdict, ProbeVerdict::Mined);
        assert_eq!(report.block_height, Some(1_234));
    }

    /// A node that has the transaction in a block makes it exist, whatever
    /// the refusals and lookups that follow say.
    #[tokio::test]
    async fn should_accept_a_refused_transaction_one_node_has_in_a_block() {
        let refused_then_seen_mined = ScriptedNodes::with_lookups(
            vec![(Some("a"), refused()), (Some("b"), refused())],
            vec![
                (Some("c"), in_block()),
                (Some("d"), LookupAnswer::NotFound),
                (Some("e"), LookupAnswer::NotFound),
            ],
        );

        let report = probe_with(&refused_then_seen_mined, &transaction()).await;
        assert_eq!(report.verdict, ProbeVerdict::Accepted);
        assert_eq!(
            report.in_block_by,
            BTreeSet::from(["c".to_string()]),
            "reported, for the resolver to add up across probes"
        );
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

        assert_eq!(answer(Vec::new(), 0, 0), in_mempool());
        assert_eq!(
            answer(vec![1; 32], 100, 3),
            LookupAnswer::Known {
                mined: true,
                height: Some(100),
            },
            "the node's block height is kept"
        );
        assert!(matches!(
            answer(vec![1; 32], 0, 0),
            LookupAnswer::Unknown { .. }
        ));
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

    #[tokio::test]
    async fn should_not_look_up_a_transaction_nobody_refused() {
        let nodes = ScriptedNodes::new(vec![(Some("a"), NodeVerdict::Accepted)]);

        probe_with(&nodes, &transaction()).await;

        assert_eq!(nodes.lookups_made(), 0);
    }

    /// Refused for its inputs yet still in a node's mempool (a double-spend
    /// race the node lost locally): not mined — keep asking.
    #[tokio::test]
    async fn should_accept_a_refused_transaction_a_node_holds_in_its_mempool() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), refused()), (Some("b"), refused())],
            vec![(Some("c"), in_mempool())],
        );

        assert_eq!(
            probe_with(&nodes, &transaction()).await.verdict,
            ProbeVerdict::Accepted
        );
    }
}
