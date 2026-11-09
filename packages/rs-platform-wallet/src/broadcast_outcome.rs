//! Classifying the outcome of a state-transition broadcast / result wait.
//!
//! Shared by every Platform money-moving path that must keep "definitely
//! rejected, safe to retry" apart from "ambiguous, may have executed, do
//! not retry". Shielded spends that consume notes broadcast through
//! [`broadcast_tracking_delivery`], which keeps every dispatch's outcome so an
//! earlier dispatch that may have been delivered is never hidden by a later
//! refusal; shields into the pool and masternode credit withdrawals classify
//! the SDK's final error with [`broadcast_definitely_failed`]. The shapes are
//! SDK-transport facts, not operation-specific, so they live here rather than
//! behind the `shielded` feature.

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
/// later attempt's final error, whatever its code. That is why shielded
/// spends do not use this classification but [`broadcast_tracking_delivery`],
/// which reports such a spend unconfirmed: a spend reported failed while it
/// may still execute could be paid again with other notes. The callers left
/// here are shields into the pool, which move funds to the wallet's own
/// notes, and masternode credit withdrawals.
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

/// How many single dispatches [`broadcast_tracking_delivery`] makes at most:
/// the SDK's default budget, one dispatch and five retries.
#[cfg(feature = "shielded")]
const SPEND_BROADCAST_DISPATCHES: usize = 6;

/// What the dispatches of one shielded spend's broadcast proved about its
/// delivery ([`broadcast_tracking_delivery`]).
#[cfg(feature = "shielded")]
#[derive(Debug)]
pub(crate) enum BroadcastDelivery {
    /// A node admitted the transition, or already held it (`AlreadyExists`).
    Admitted,
    /// No dispatch proved the transition was left out of every mempool: a
    /// dispatch failed in transport, or in any other way that allows its
    /// request to have been delivered. It may execute, so the caller waits
    /// for its result, and an unconfirmed outcome keeps the notes reserved.
    MaybeDelivered(dash_sdk::Error),
    /// The transition never left the wallet: the first dispatch to reach a
    /// node was refused with Platform's consensus verdict, after no dispatch
    /// that could have delivered it, or there was no address to send it to,
    /// or the SDK refused it before sending. Nothing holds its bytes but the
    /// wallet, so it can never execute.
    NeverDelivered(dash_sdk::Error),
}

/// Broadcast a shielded spend `state_transition` in single dispatches (no
/// SDK retry inside a dispatch) at most [`SPEND_BROADCAST_DISPATCHES`]
/// times, keeping what every dispatch proved: the SDK's own retry loop
/// returns only its last error, which cannot show that an earlier attempt
/// was delivered with its response lost.
///
/// Any dispatch that failed in a way that allows delivery (every transport
/// error, whatever its gRPC code, a timeout, a cause-less broadcast
/// envelope, an unknown error) makes the whole broadcast
/// [`BroadcastDelivery::MaybeDelivered`]: a later consensus rejection does
/// not override it, as a lagging node can refuse a transition another one
/// admitted. Only a rejection or a refusal with nothing possibly delivered
/// before it is [`BroadcastDelivery::NeverDelivered`].
#[cfg(feature = "shielded")]
pub(crate) async fn broadcast_tracking_delivery(
    sdk: &dash_sdk::Sdk,
    state_transition: &dpp::state_transition::StateTransition,
) -> BroadcastDelivery {
    use dash_sdk::dapi_client::RequestSettings;
    use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
    use dash_sdk::platform::transition::put_settings::PutSettings;

    let single_dispatch = PutSettings {
        request_settings: RequestSettings {
            retries: Some(0),
            ..RequestSettings::default()
        },
        ..PutSettings::default()
    };
    track_delivery(|| state_transition.broadcast(sdk, Some(single_dispatch))).await
}

/// The classification [`broadcast_tracking_delivery`] makes, over the
/// outcomes `dispatch` returns, one call per single dispatch.
#[cfg(feature = "shielded")]
async fn track_delivery<F, Fut>(mut dispatch: F) -> BroadcastDelivery
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), dash_sdk::Error>>,
{
    use dash_sdk::dapi_client::DapiClientError;

    let mut maybe_delivered: Option<dash_sdk::Error> = None;
    for _ in 0..SPEND_BROADCAST_DISPATCHES {
        let error = match dispatch().await {
            Ok(()) | Err(dash_sdk::Error::AlreadyExists(_)) => return BroadcastDelivery::Admitted,
            Err(error) => error,
        };
        let nothing_sent = matches!(
            error,
            dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses)
                | dash_sdk::Error::ProtocolVersionNotActive { .. }
        ) || matches!(
            &error,
            dash_sdk::Error::NoAvailableAddressesToRetry(inner)
                if matches!(
                    **inner,
                    dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses)
                )
        );
        if carries_consensus_rejection(&error) || nothing_sent {
            // Final: a verdict, or nowhere left to send it.
            return match maybe_delivered {
                Some(_) => BroadcastDelivery::MaybeDelivered(error),
                None => BroadcastDelivery::NeverDelivered(error),
            };
        }
        maybe_delivered = Some(error);
    }
    match maybe_delivered {
        Some(error) => BroadcastDelivery::MaybeDelivered(error),
        // Unreachable with a non-zero budget: every pass sets it or returns.
        None => BroadcastDelivery::MaybeDelivered(dash_sdk::Error::Generic(
            "no broadcast dispatch was made".to_string(),
        )),
    }
}

#[cfg(all(test, feature = "shielded"))]
mod tests {
    use super::*;
    use dash_sdk::dapi_client::transport::TransportError;
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dapi_grpc::tonic::{Code, Status};
    use std::sync::Mutex;

    /// Another error of the same shape as `error`, which is not `Clone`.
    fn copy(error: &dash_sdk::Error) -> dash_sdk::Error {
        match error {
            dash_sdk::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
                status,
            ))) => grpc(status.code()),
            dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses) => {
                dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses)
            }
            dash_sdk::Error::NoAvailableAddressesToRetry(inner) => {
                dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(copy(inner)))
            }
            dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(_)) => rejection(),
            other => dash_sdk::Error::Generic(other.to_string()),
        }
    }

    fn grpc(code: Code) -> dash_sdk::Error {
        dash_sdk::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
            Status::new(code, "test"),
        )))
    }

    /// A consensus rejection, as DAPI attaches one to a refused broadcast.
    fn rejection() -> dash_sdk::Error {
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::basic::UnsupportedFeatureError;
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            dpp::consensus::ConsensusError::BasicError(BasicError::UnsupportedFeatureError(
                UnsupportedFeatureError::new("test".to_string(), 1),
            )),
        )))
    }

    /// What [`track_delivery`] makes of dispatches whose outcome `outcome`
    /// gives by dispatch number, from 0, and how many dispatches it made.
    async fn tracked(
        outcome: impl Fn(usize) -> Result<(), dash_sdk::Error>,
    ) -> (BroadcastDelivery, usize) {
        let dispatched = Mutex::new(0usize);
        let delivery = track_delivery(|| {
            let mut count = dispatched.lock().expect("count");
            let result = outcome(*count);
            *count += 1;
            std::future::ready(result)
        })
        .await;
        let dispatched = *dispatched.lock().expect("count");
        (delivery, dispatched)
    }

    /// An attempt admitted with its response lost, then retries refused for
    /// any reason, leaves the broadcast maybe delivered: the last error
    /// alone cannot show the transition never left.
    #[tokio::test]
    async fn should_keep_an_earlier_ambiguous_dispatch_over_later_refusals() {
        for lost in [Code::Unavailable, Code::DeadlineExceeded] {
            for refusal in [
                grpc(Code::ResourceExhausted),
                grpc(Code::Unimplemented),
                grpc(Code::InvalidArgument),
                rejection(),
            ] {
                let refusal = &refusal;
                let (delivery, _) = tracked(|dispatch| match dispatch {
                    0 => Err(grpc(lost)),
                    _ => Err(copy(refusal)),
                })
                .await;
                assert!(
                    matches!(delivery, BroadcastDelivery::MaybeDelivered(_)),
                    "{lost:?} then a refusal: {delivery:?}"
                );
            }
        }
    }

    /// Every transport-level failure may have delivered the transition,
    /// whatever its code, so a broadcast that only ever fails so is maybe
    /// delivered after the whole budget.
    #[tokio::test]
    async fn should_treat_every_transport_failure_as_maybe_delivered() {
        for code in [
            Code::Unavailable,
            Code::ResourceExhausted,
            Code::Unimplemented,
            Code::InvalidArgument,
            Code::Internal,
        ] {
            let (delivery, dispatched) = tracked(|_| Err(grpc(code))).await;
            assert!(
                matches!(delivery, BroadcastDelivery::MaybeDelivered(_)),
                "{code:?}: {delivery:?}"
            );
            assert_eq!(dispatched, SPEND_BROADCAST_DISPATCHES, "{code:?}");
        }
    }

    /// Only a consensus verdict, or nowhere to send the transition, before
    /// any dispatch that could have delivered it shows it never left.
    #[tokio::test]
    async fn should_count_a_broadcast_never_delivered_only_without_an_earlier_ambiguity() {
        for first in [
            rejection(),
            dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses),
            dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(
                dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses),
            )),
        ] {
            let first = &first;
            let (delivery, dispatched) = tracked(|_| Err(copy(first))).await;
            assert!(
                matches!(delivery, BroadcastDelivery::NeverDelivered(_)),
                "{delivery:?}"
            );
            assert_eq!(dispatched, 1);
        }
    }

    /// A dispatch a node admitted, or already holds, is admitted, after any
    /// number of failed ones.
    #[tokio::test]
    async fn should_count_an_admitted_or_known_dispatch_as_admitted() {
        let (delivery, _) = tracked(|dispatch| match dispatch {
            0 => Err(grpc(Code::Unavailable)),
            _ => Ok(()),
        })
        .await;
        assert!(
            matches!(delivery, BroadcastDelivery::Admitted),
            "{delivery:?}"
        );
        let (delivery, _) = tracked(|dispatch| match dispatch {
            0 => Err(grpc(Code::DeadlineExceeded)),
            _ => Err(dash_sdk::Error::AlreadyExists("in the mempool".to_string())),
        })
        .await;
        assert!(
            matches!(delivery, BroadcastDelivery::Admitted),
            "{delivery:?}"
        );
    }
}
