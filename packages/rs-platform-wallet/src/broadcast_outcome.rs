//! Classifying the outcome of a state-transition broadcast / result wait.
//!
//! Shared by every Platform money-moving path that must keep "definitely
//! rejected, safe to retry" apart from "ambiguous, may have executed, do
//! not retry" (shielded spends, masternode credit withdrawals). The shapes
//! are SDK-transport facts, not operation-specific, so they live here
//! rather than behind the `shielded` feature.

/// Whether an SDK error carries Platform's own consensus verdict on the
/// transition. Two shapes qualify:
///
/// - `Error::Protocol(ProtocolError::ConsensusError(_))` — DAPI attached the
///   serialized consensus error as gRPC metadata
///   (`dash-serialized-consensus-error-bin`), which the dapi-client decodes
///   on any failed request. This is how a CheckTx rejection of the
///   transition surfaces from `broadcast()` (rs-dapi's
///   `map_broadcast_error` decodes the consensus error from Tenderdash's
///   `info` field and `TenderdashStatus` re-attaches it as metadata);
/// - a `StateTransitionBroadcastError` whose `cause` deserialized from
///   non-empty consensus `data` — the wait-stream error envelope for a
///   transition Platform executed and rejected on its merits.
///
/// Recurses through a `NoAvailableAddressesToRetry` envelope, mirroring
/// [`crate::error::as_address_invalid_nonce`].
///
/// Only these prove the transition was evaluated and REJECTED. Everything
/// else — transport errors, timeouts, `AlreadyExists` (which proves the
/// opposite: the transition is already in the mempool or on chain),
/// DAPI-internal failures, cause-less broadcast envelopes (the shape DAPI
/// uses for its own wait-side timeouts) — leaves the outcome unknown.
pub(crate) fn carries_consensus_rejection(err: &dash_sdk::Error) -> bool {
    match err {
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(_)) => true,
        dash_sdk::Error::StateTransitionBroadcastError(e) => e.cause.is_some(),
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => carries_consensus_rejection(inner),
        _ => false,
    }
}

/// Whether a failed `broadcast()` call DEFINITIVELY left the transition
/// out of every mempool, so any note reservations may be released and the
/// caller may rebuild and retry:
///
/// - a consensus verdict ([`carries_consensus_rejection`]): CheckTx
///   evaluated the transition and refused it;
/// - a gRPC response whose status code is a server-side rejection or a
///   connection-establishment failure. `Unavailable` is the common shape
///   of a connect-refused/offline attempt — classifying it as definitive
///   keeps the no-network failure's notes immediately re-spendable
///   instead of stranding them until the next restart — and rejection
///   codes (`InvalidArgument`, `ResourceExhausted` = mempool full, …) are
///   verdicts that the tx was refused admission;
/// - no usable DAPI addresses at all (nothing was ever sent).
///
/// `Unavailable` is NOT an absolute never-delivered guarantee: HTTP/2
/// stream resets after the request bytes left can surface the same code,
/// and the dapi-client's cross-address retry only retains the LAST
/// transport error, so an earlier-attempt delivery can hide behind a
/// later attempt's `Unavailable`. Releasing the notes in that residual
/// window is still fund-safe — the authoritative no-reuse guarantee is
/// the on-chain nullifier set, so a re-selected note at worst wastes a
/// ~30 s proof on a nullifier-already-used rejection (see the
/// `finalize_pending` downgrade rationale in `unshield`); never fund
/// loss. The trade is deliberate: UX for the dominant offline case over
/// strict conservatism in a rare race.
///
/// Everything else leaves the outcome unknown and the caller must fall
/// through to the result wait instead of failing: `AlreadyExists` proves
/// the tx IS in the mempool or on chain (a lost-ACK attempt was re-sent
/// by the dapi-client retry and hit tenderdash's dedupe), and
/// timeout/cancellation/no-response shapes (`TimeoutReached`,
/// `Cancelled`, gRPC `DeadlineExceeded`/`Cancelled`, plus
/// `Internal`/`Unknown`/`Aborted`/`DataLoss`, which DAPI also uses for
/// its own tenderdash-side failures that can postdate delivery) allow
/// the request to have outlived its lost ACK.
pub(crate) fn broadcast_definitely_failed(e: &dash_sdk::Error) -> bool {
    use dash_sdk::dapi_client::transport::TransportError;
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dapi_grpc::tonic::Code;

    fn status_is_verdict(t: &TransportError) -> bool {
        let TransportError::Grpc(status) = t;
        !matches!(
            status.code(),
            Code::DeadlineExceeded
                | Code::Cancelled
                | Code::Unknown
                | Code::Internal
                | Code::Aborted
                | Code::DataLoss
        )
    }

    if carries_consensus_rejection(e) {
        return true;
    }
    match e {
        dash_sdk::Error::AlreadyExists(_) => false,
        dash_sdk::Error::DapiClientError(DapiClientError::Transport(t)) => status_is_verdict(t),
        dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses) => true,
        dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(t)) => {
            status_is_verdict(t)
        }
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => broadcast_definitely_failed(inner),
        _ => false,
    }
}
