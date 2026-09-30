//! Asking Core nodes directly whether a signed transaction can still land.
//!
//! The P2P network has no negative signal — modern Dash Core removed BIP61
//! `reject` — so a transaction whose SPV broadcast saw no acceptance signal
//! (`BroadcastResult::Uncertain` → [`BroadcastError::MaybeSent`]) stays
//! ambiguous for as long as nobody asks. For a valid transaction that is the
//! right answer: it may still be relayed and mined. For one that can never
//! land — its input does not exist, or is already spent by a final
//! transaction — nothing ends the ambiguity: no block revisits it, dash-spv
//! keeps rebroadcasting it, and its inputs stay fenced indefinitely.
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
//! The second row is why a rejection alone is never enough to call a
//! transaction dead: Core recognises a mined transaction only while one of its
//! outputs is still in the UTXO set, and otherwise evaluates it like a new one
//! whose inputs are gone (measured on testnet 2026-09-29: `e758f06a…`, block
//! 1560020, both outputs spent → `-25`). Before a dead verdict the probe
//! therefore asks nodes whether they know the txid at all (DAPI
//! `getTransaction`, served from the evonode's transaction index); a node that
//! knows it rules out a dead verdict.
//!
//! Mined needs two distinct nodes reporting the transaction in a block
//! ([`MINED_QUORUM`]); one such report makes it accepted — it exists — and the
//! second opinion is asked by lookup, not by resubmitting. On a network with
//! a single reachable evonode (a devnet pinned to one address) nothing is
//! ever mined by probe, only accepted; the wallet's own block processing
//! settles it. A block two nodes saw that a reorg drops before its ChainLock
//! (seconds on Dash) is not revisited: Mined is final.
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

use async_trait::async_trait;
use dash_sdk::dapi_client::transport::TransportError;
use dash_sdk::dapi_client::{DapiClientError, DapiRequestExecutor, RequestSettings};
use dash_sdk::dapi_grpc::core::v0::{BroadcastTransactionRequest, GetTransactionRequest};
use dash_sdk::dapi_grpc::tonic::Code;
use dashcore::consensus;
use dashcore::{Transaction, Txid};

/// Core reject reasons that prove the transaction can never be mined as it
/// stands.
///
/// - `bad-txns-inputs-missingorspent`: an input is neither in the UTXO set
///   nor created by anything in this node's mempool. `Missing inputs` is the
///   older spelling of the same check.
/// - `tx-txlock-conflict`: an input is already spent by an InstantSend-locked
///   transaction, which is final.
///
/// A node's `-25` alone is never taken as proof. It is also what a node that
/// has not yet seen one of the transaction's unconfirmed parents answers
/// (the wallet-side resolver only probes chain roots for that reason), what a
/// node lagging behind the tip answers (hence [`DEAD_QUORUM`]: two distinct
/// nodes), and what a mined transaction whose outputs are all spent gets
/// (hence the txid lookup in [`confirm_dead`]).
///
/// `txn-mempool-conflict` is deliberately absent: it means another,
/// unconfirmed and not IS-locked, transaction spends the same input, and
/// either one may still be mined — no verdict.
const DEAD_REASONS: &[&str] = &[
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
    /// The node proved the transaction can never be mined as it stands.
    Dead { reason: String },
    /// No verdict: transport failure, timeout, or a rejection that proves
    /// nothing (such as `txn-mempool-conflict`).
    Unknown { reason: String },
}

/// Classify a failed `broadcastTransaction` gRPC response.
pub(crate) fn classify_failed_submission(code: Code, message: &str) -> NodeVerdict {
    if code == Code::AlreadyExists {
        return NodeVerdict::Mined;
    }
    let lowered = message.to_ascii_lowercase();
    if DEAD_REASONS.iter().any(|reason| lowered.contains(reason)) {
        return NodeVerdict::Dead {
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
    /// A node holds the transaction in its mempool. Not settlement: it can
    /// still expire or lose to a conflict, so the resolver keeps asking.
    Accepted,
    /// Two different nodes have the transaction in a block. Final — there is
    /// nothing left to ask, even if this wallet has not seen that block yet.
    Mined,
    /// Two different nodes refused it for a reason in [`DEAD_REASONS`]
    /// (missing or spent inputs, or a conflict with an InstantSend-locked
    /// spend), and two different nodes answered `getTransaction` with
    /// NotFound: it is not in a block and can never be mined.
    Dead { reason: String },
    /// Not enough evidence either way; the transaction stays ambiguous.
    Unresolved { reason: String },
}

/// Whether one node knows a txid (DAPI `getTransaction`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LookupAnswer {
    /// The node has the transaction; `mined` when it is in a block.
    Known { mined: bool },
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

/// How many txid lookups one probe may spend confirming a dead verdict.
const MAX_LOOKUPS_PER_PROBE: usize = 4;

/// How many *distinct* nodes must independently prove a transaction dead.
///
/// One node is not enough: a node lagging behind the tip has not seen a
/// recently confirmed parent and answers `missingorspent` for a transaction
/// that is perfectly valid. Two unrelated evonodes lagging on the same parent
/// at the same time is far less likely, and the cost of waiting for a second
/// opinion is one more request.
const DEAD_QUORUM: usize = 2;

/// How many *distinct* nodes must report a transaction in a block before it
/// counts as mined — final, never asked about again, its children probed in
/// its place. One node on a stale fork, or one whose block is about to be
/// reorged out, would otherwise settle it on false evidence. A single such
/// answer still shows the transaction exists: it counts as accepted.
const MINED_QUORUM: usize = 2;

/// Resubmits a transaction whose broadcast outcome is unknown and turns the
/// nodes' answers into a verdict.
///
/// Resubmitting is safe: it is the same signed bytes, so a node either takes
/// a transaction it did not have — an extra delivery route — or tells us why
/// it will not. It can never create a second payment.
#[async_trait]
pub trait AcceptanceProbe: Send + Sync + 'static {
    async fn probe(&self, transaction: &Transaction) -> ProbeVerdict;
}

/// Evidence rules shared by every probe: a node holding the transaction in its
/// mempool makes it accepted; one reporting it in a block rules out dead and
/// sends the probe to lookups for a second opinion ([`MINED_QUORUM`] distinct
/// nodes make it mined); dead needs [`DEAD_QUORUM`] distinct nodes refusing
/// it for its inputs **and** [`DEAD_QUORUM`] distinct nodes not knowing its
/// txid, with none knowing it; give up after [`MAX_SUBMISSIONS_PER_PROBE`]
/// submissions and [`MAX_LOOKUPS_PER_PROBE`] lookups.
pub(crate) async fn probe_with(
    submitter: &dyn NodeSubmitter,
    transaction: &Transaction,
) -> ProbeVerdict {
    let mut dead_nodes: HashSet<String> = HashSet::new();
    let mut dead_reason: Option<String> = None;
    let mut mined = MinedEvidence::default();
    let mut last_other = String::from("no submission was answered");

    for _ in 0..MAX_SUBMISSIONS_PER_PROBE {
        let (node, verdict) = submitter.submit(transaction).await;
        tracing::debug!(
            txid = %transaction.txid(),
            node = node.as_deref().unwrap_or("unidentified"),
            ?verdict,
            "broadcast probe: node answered"
        );
        match verdict {
            NodeVerdict::Accepted => return ProbeVerdict::Accepted,
            NodeVerdict::Mined => {
                // In a block by this node's word: not dead. A second opinion
                // comes from a lookup, not from sending it again.
                mined.add(node);
                return confirm_by_lookup(submitter, &transaction.txid(), mined, None).await;
            }
            NodeVerdict::Dead { reason } => {
                // A dead verdict from an unidentified node cannot count
                // towards the quorum — it might be the node we already heard.
                if let Some(node) = node {
                    dead_nodes.insert(node);
                }
                dead_reason.get_or_insert(reason);
                if dead_nodes.len() >= DEAD_QUORUM {
                    return confirm_by_lookup(submitter, &transaction.txid(), mined, dead_reason)
                        .await;
                }
            }
            NodeVerdict::Unknown { reason } => {
                last_other = reason;
            }
        }
    }

    ProbeVerdict::Unresolved {
        reason: match dead_reason {
            Some(reason) => format!(
                "{} of {DEAD_QUORUM} nodes needed proved it dead ({reason}); last other answer: {last_other}",
                dead_nodes.len()
            ),
            None => last_other,
        },
    }
}

/// Nodes that reported the transaction in a block, by submission or lookup.
#[derive(Default)]
struct MinedEvidence {
    nodes: HashSet<String>,
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
/// mempool makes it accepted; in a block, counts towards [`MINED_QUORUM`] and
/// rules out dead. With `refused` — the dead quorum of refusals is met — only
/// [`DEAD_QUORUM`] distinct nodes not knowing the txid, and none knowing it,
/// make it dead: a rejection for missing or spent inputs is also what a
/// *mined* transaction whose outputs are all spent gets (see the module docs).
async fn confirm_by_lookup(
    submitter: &dyn NodeSubmitter,
    txid: &Txid,
    mut mined: MinedEvidence,
    refused: Option<String>,
) -> ProbeVerdict {
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
            LookupAnswer::Known { mined: true } => {
                if mined.add(node) {
                    return ProbeVerdict::Mined;
                }
            }
            LookupAnswer::Known { mined: false } => return ProbeVerdict::Accepted,
            LookupAnswer::NotFound => {
                if let Some(node) = node {
                    not_found.insert(node);
                }
                if let Some(reason) = &refused {
                    if !mined.seen && not_found.len() >= DEAD_QUORUM {
                        return ProbeVerdict::Dead {
                            reason: reason.clone(),
                        };
                    }
                }
            }
            LookupAnswer::Unknown { reason } => last_unknown = reason,
        }
    }
    if mined.seen {
        return ProbeVerdict::Accepted;
    }
    ProbeVerdict::Unresolved {
        reason: match refused {
            Some(reason) => format!(
                "refused as {reason}, but only {} of {DEAD_QUORUM} nodes confirmed the txid is unknown; last lookup answer: {last_unknown}",
                not_found.len()
            ),
            None => last_unknown,
        },
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
            Ok(response) => {
                let mined = response.inner.height > 0 || response.inner.confirmations > 0;
                (
                    Some(response.address.to_string()),
                    LookupAnswer::Known { mined },
                )
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
    async fn probe(&self, transaction: &Transaction) -> ProbeVerdict {
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
    /// default to two distinct nodes not knowing the txid, so a dead quorum
    /// is confirmed unless a test scripts otherwise.
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

    fn dead() -> NodeVerdict {
        NodeVerdict::Dead {
            reason: "bad-txns-inputs-missingorspent".to_string(),
        }
    }

    fn unknown() -> NodeVerdict {
        NodeVerdict::Unknown {
            reason: "Unavailable: connection refused".to_string(),
        }
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
                dead(),
            ),
            // -25, rs-dapi.
            (
                Code::FailedPrecondition,
                "Transaction is rejected: bad-txns-inputs-missingorspent",
                dead(),
            ),
            // -26 against an IS-locked spend: final.
            (
                Code::FailedPrecondition,
                "Transaction is rejected: tx-txlock-conflict",
                NodeVerdict::Dead {
                    reason: "Transaction is rejected: tx-txlock-conflict".to_string(),
                },
            ),
            // -26 against an unconfirmed spend: not final.
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
            let verdict = classify_failed_submission(code, message);
            match (&verdict, &expected) {
                (NodeVerdict::Dead { .. }, NodeVerdict::Dead { .. }) => {}
                _ => assert_eq!(verdict, expected, "{code:?} {message}"),
            }
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
            (Some("a"), dead()),
            (Some("b"), NodeVerdict::Accepted),
        ]);

        assert_eq!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Accepted
        );
        assert_eq!(nodes.submissions(), 2);
    }

    #[tokio::test]
    async fn should_call_a_transaction_dead_only_when_two_distinct_nodes_prove_it() {
        let nodes = ScriptedNodes::new(vec![(Some("a"), dead()), (Some("b"), dead())]);

        assert!(matches!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Dead { .. }
        ));
        assert_eq!(nodes.submissions(), 2);
    }

    /// A lagging node answers `missingorspent` for a valid transaction. Hearing
    /// the same node twice must not look like two opinions.
    #[tokio::test]
    async fn should_not_count_the_same_node_twice_towards_the_dead_quorum() {
        let nodes = ScriptedNodes::new(vec![
            (Some("a"), dead()),
            (Some("a"), dead()),
            (Some("a"), dead()),
            (Some("a"), dead()),
        ]);

        assert!(matches!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Unresolved { .. }
        ));
        assert_eq!(nodes.submissions(), MAX_SUBMISSIONS_PER_PROBE);
    }

    #[tokio::test]
    async fn should_not_count_an_unidentified_node_towards_the_dead_quorum() {
        let nodes = ScriptedNodes::new(vec![(Some("a"), dead()), (None, dead())]);

        assert!(matches!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Unresolved { .. }
        ));
    }

    #[tokio::test]
    async fn should_stay_unresolved_when_nodes_only_refuse_or_fail() {
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

        assert!(matches!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Unresolved { .. }
        ));
        assert_eq!(nodes.submissions(), MAX_SUBMISSIONS_PER_PROBE);
    }

    #[tokio::test]
    async fn should_reach_the_quorum_across_intervening_failures() {
        let nodes = ScriptedNodes::new(vec![
            (Some("a"), dead()),
            (Some("b"), unknown()),
            (Some("c"), dead()),
        ]);

        assert!(matches!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Dead { .. }
        ));
        assert_eq!(nodes.submissions(), 3);
    }

    /// Core answers `missingorspent` for a mined transaction whose outputs
    /// are all spent (testnet `e758f06a…`, 2026-09-29). A node that still
    /// knows the txid in a block must turn the dead quorum into Mined.
    #[tokio::test]
    async fn should_report_mined_for_a_refused_transaction_a_node_has_in_a_block() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), dead()), (Some("b"), dead())],
            vec![
                (Some("c"), LookupAnswer::Known { mined: true }),
                (Some("d"), LookupAnswer::Known { mined: true }),
            ],
        );

        assert_eq!(
            probe_with(&nodes, &transaction()).await,
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
            vec![(Some("b"), LookupAnswer::Known { mined: true })],
        );
        let same_twice = ScriptedNodes::with_lookups(
            vec![(Some("a"), NodeVerdict::Mined)],
            vec![(Some("a"), LookupAnswer::Known { mined: true })],
        );

        assert_eq!(probe_with(&two, &transaction()).await, ProbeVerdict::Mined);
        assert_eq!(two.submissions(), 1, "the second opinion is a lookup");
        assert_eq!(
            probe_with(&same_twice, &transaction()).await,
            ProbeVerdict::Accepted,
            "one node's word: it exists, not that it is final"
        );
    }

    /// A node that has the transaction in a block rules out dead, whatever
    /// the refusals and lookups that follow say.
    #[tokio::test]
    async fn should_never_call_dead_a_transaction_a_node_has_in_a_block() {
        let refused_then_seen_mined = ScriptedNodes::with_lookups(
            vec![(Some("a"), dead()), (Some("b"), dead())],
            vec![
                (Some("c"), LookupAnswer::Known { mined: true }),
                (Some("d"), LookupAnswer::NotFound),
                (Some("e"), LookupAnswer::NotFound),
            ],
        );
        let mined_then_refused = ScriptedNodes::new(vec![
            (Some("a"), NodeVerdict::Mined),
            (Some("b"), dead()),
            (Some("c"), dead()),
        ]);

        assert_eq!(
            probe_with(&refused_then_seen_mined, &transaction()).await,
            ProbeVerdict::Accepted
        );
        assert_eq!(
            probe_with(&mined_then_refused, &transaction()).await,
            ProbeVerdict::Accepted
        );
    }

    #[tokio::test]
    async fn should_confirm_dead_only_when_two_distinct_nodes_do_not_know_the_txid() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), dead()), (Some("b"), dead())],
            vec![
                (Some("c"), LookupAnswer::NotFound),
                (Some("c"), LookupAnswer::NotFound),
                (None, LookupAnswer::NotFound),
                (
                    Some("d"),
                    LookupAnswer::Unknown {
                        reason: "Unavailable".to_string(),
                    },
                ),
            ],
        );

        assert!(matches!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Unresolved { .. }
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
    /// race the node lost locally): not dead, not mined — keep asking.
    #[tokio::test]
    async fn should_accept_a_refused_transaction_a_node_holds_in_its_mempool() {
        let nodes = ScriptedNodes::with_lookups(
            vec![(Some("a"), dead()), (Some("b"), dead())],
            vec![(Some("c"), LookupAnswer::Known { mined: false })],
        );

        assert_eq!(
            probe_with(&nodes, &transaction()).await,
            ProbeVerdict::Accepted
        );
    }
}
