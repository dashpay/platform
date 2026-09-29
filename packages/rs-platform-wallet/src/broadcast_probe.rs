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
//! | already in a block                            | `-27 Transaction already in block chain`  |
//! | spends an input that does not exist           | `-25 bad-txns-inputs-missingorspent`      |
//! | spends an input an IS-locked tx already spent | `-26 tx-txlock-conflict`                  |
//!
//! The two DAPI implementations surface those codes differently — the JS
//! server maps `-25` to `InvalidArgument`, rs-dapi to `FailedPrecondition` —
//! so the classification keys on Core's reason text, which both pass through
//! in the status message, and on `AlreadyExists` for `-27`, which both agree
//! on.
//!
//! [`BroadcastError::MaybeSent`]: crate::broadcaster::BroadcastError::MaybeSent

use std::collections::HashSet;

use async_trait::async_trait;
use dashcore::Transaction;

/// Core reject reasons that prove the transaction can never be mined as it
/// stands.
///
/// - `bad-txns-inputs-missingorspent`: an input is neither in the UTXO set
///   nor created by anything in this node's mempool. `Missing inputs` is the
///   older spelling of the same check.
/// - `tx-txlock-conflict`: an input is already spent by an InstantSend-locked
///   transaction, which is final.
///
/// `-25` has two false positives that the *caller* must rule out — a node
/// that has not yet seen one of our own unconfirmed parents, and a node
/// lagging behind the tip. See [`AcceptanceProbe`] for how the probe guards
/// against the second, and the wallet-side resolver for the first.
const DEAD_REASONS: &[&str] = &[
    "bad-txns-inputs-missingorspent",
    "missing inputs",
    "tx-txlock-conflict",
];

/// Core reject reasons that say this node refused the transaction without
/// proving it can never land: `txn-mempool-conflict` means another,
/// unconfirmed and not IS-locked, transaction spends the same input, and
/// either one may still be mined.
const REFUSED_REASONS: &[&str] = &["txn-mempool-conflict"];

/// What a single Core node said when handed a signed transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeVerdict {
    /// The node holds the transaction — it accepted it into its mempool now,
    /// already had it there, or has it in a block.
    Accepted,
    /// The node proved the transaction can never be mined as it stands.
    Dead { reason: String },
    /// The node refused the transaction without proving it dead.
    Refused { reason: String },
    /// No verdict: transport failure, timeout, or an unrecognised rejection.
    Unknown { reason: String },
}

/// Classify a failed `broadcastTransaction` gRPC response.
pub(crate) fn classify_failed_submission(
    code: dash_sdk::dapi_grpc::tonic::Code,
    message: &str,
) -> NodeVerdict {
    use dash_sdk::dapi_grpc::tonic::Code;

    if code == Code::AlreadyExists {
        return NodeVerdict::Accepted;
    }
    let lowered = message.to_ascii_lowercase();
    if DEAD_REASONS.iter().any(|reason| lowered.contains(reason)) {
        return NodeVerdict::Dead {
            reason: message.to_string(),
        };
    }
    if REFUSED_REASONS
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
    /// A node holds the transaction: it is in a mempool or a block.
    Accepted,
    /// Two different nodes proved the transaction can never be mined.
    Dead { reason: String },
    /// Not enough evidence either way; the transaction stays ambiguous.
    Unresolved { reason: String },
}

/// Submits a signed transaction to one Core node and reports which node
/// answered and what it said. A seam so the probe's evidence rules can be
/// tested without a network.
#[async_trait]
pub(crate) trait NodeSubmitter: Send + Sync {
    async fn submit(&self, transaction: &Transaction) -> (Option<String>, NodeVerdict);
}

/// How many submissions one probe may spend looking for a verdict.
const MAX_SUBMISSIONS_PER_PROBE: usize = 4;

/// How many *distinct* nodes must independently prove a transaction dead.
///
/// One node is not enough: a node lagging behind the tip has not seen a
/// recently confirmed parent and answers `missingorspent` for a transaction
/// that is perfectly valid. Two unrelated evonodes lagging on the same parent
/// at the same time is far less likely, and the cost of waiting for a second
/// opinion is one more request.
const DEAD_QUORUM: usize = 2;

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

/// Evidence rules shared by every probe: stop at the first node that holds
/// the transaction, call it dead only when [`DEAD_QUORUM`] distinct nodes say
/// so, and give up after [`MAX_SUBMISSIONS_PER_PROBE`] submissions.
pub(crate) async fn probe_with(
    submitter: &dyn NodeSubmitter,
    transaction: &Transaction,
) -> ProbeVerdict {
    let mut dead_nodes: HashSet<String> = HashSet::new();
    let mut dead_reason: Option<String> = None;
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
            NodeVerdict::Dead { reason } => {
                // A dead verdict from an unidentified node cannot count
                // towards the quorum — it might be the node we already heard.
                if let Some(node) = node {
                    dead_nodes.insert(node);
                }
                dead_reason.get_or_insert(reason);
                if dead_nodes.len() >= DEAD_QUORUM {
                    return ProbeVerdict::Dead {
                        reason: dead_reason.unwrap_or_default(),
                    };
                }
            }
            NodeVerdict::Refused { reason } | NodeVerdict::Unknown { reason } => {
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

/// Submits through DAPI's `broadcastTransaction` — a Core `sendrawtransaction`
/// on a random evonode per request.
pub(crate) struct DapiNodeSubmitter {
    sdk: std::sync::Arc<dash_sdk::Sdk>,
}

#[async_trait]
impl NodeSubmitter for DapiNodeSubmitter {
    async fn submit(&self, transaction: &Transaction) -> (Option<String>, NodeVerdict) {
        use dash_sdk::dapi_client::transport::TransportError;
        use dash_sdk::dapi_client::{DapiClientError, DapiRequestExecutor, RequestSettings};
        use dash_sdk::dapi_grpc::core::v0::BroadcastTransactionRequest;

        let request = BroadcastTransactionRequest {
            transaction: dashcore::consensus::serialize(transaction),
            allow_high_fees: false,
            bypass_limits: false,
        };
        // One node per submission: the probe decides whether to ask another,
        // and it has to know which node each answer came from.
        let settings = RequestSettings {
            retries: Some(0),
            ..RequestSettings::default()
        };

        match self.sdk.execute(request, settings).await {
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
}

/// The production probe: resubmits through DAPI.
pub struct DapiAcceptanceProbe {
    submitter: DapiNodeSubmitter,
}

impl DapiAcceptanceProbe {
    pub fn new(sdk: std::sync::Arc<dash_sdk::Sdk>) -> Self {
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

    /// Replays scripted node answers in order and counts submissions.
    struct ScriptedNodes {
        answers: Mutex<VecDeque<(Option<&'static str>, NodeVerdict)>>,
        submissions: Mutex<usize>,
    }

    impl ScriptedNodes {
        fn new(answers: Vec<(Option<&'static str>, NodeVerdict)>) -> Self {
            Self {
                answers: Mutex::new(answers.into()),
                submissions: Mutex::new(0),
            }
        }

        fn submissions(&self) -> usize {
            *self.submissions.lock().expect("submissions")
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
                NodeVerdict::Accepted,
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
                NodeVerdict::Refused {
                    reason: "Transaction is rejected: txn-mempool-conflict".to_string(),
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
                NodeVerdict::Refused {
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
}
