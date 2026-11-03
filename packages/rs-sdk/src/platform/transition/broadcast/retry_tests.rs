//! Real DAPI and SDK retry loops over an in-memory transport, without sockets.

use super::*;
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::metadata::MetadataValue;
use dapi_grpc::tonic::{Code, ConnectError, Status};
use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportError, TransportRequest,
};
use rs_dapi_client::DapiRequestExecutor;
use rs_dapi_client::{ConnectionPool, DapiClient, Uri};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

const DAPI_PROCESSING_REJECTION: &str = "State Transition processing error. Please report faulty state transition and try to create a new state transition with different hash as a workaround.";

#[derive(Debug)]
pub(in crate::platform::transition) enum Attempt {
    Error(Status),
    Success,
    Stall,
}

struct MemoryClient;

impl TransportClient for MemoryClient {
    fn with_uri(_uri: Uri, _pool: &ConnectionPool) -> Result<Self, TransportError> {
        Ok(Self)
    }

    fn with_uri_and_settings(
        uri: Uri,
        _settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Result<Self, TransportError> {
        Self::with_uri(uri, pool)
    }
}

#[derive(Clone, Debug)]
struct MemoryBroadcast {
    request: BroadcastStateTransitionRequest,
    attempts: Arc<Mutex<VecDeque<Attempt>>>,
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl Mockable for MemoryBroadcast {}

impl TransportRequest for MemoryBroadcast {
    type Client = MemoryClient;
    type Response = BroadcastStateTransitionResponse;

    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "broadcastStateTransition"
    }

    fn classify_error(
        &self,
        error: &TransportError,
    ) -> rs_dapi_client::transport::RequestErrorClassification {
        self.request.classify_error(error)
    }

    fn execute_transport<'c>(
        self,
        _client: &'c mut Self::Client,
        _settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        self.sent
            .lock()
            .unwrap()
            .push(self.request.state_transition);
        match self.attempts.lock().unwrap().pop_front().expect("attempt") {
            Attempt::Error(status) => Box::pin(async { Err(TransportError::Grpc(status)) }),
            Attempt::Success => Box::pin(async { Ok(BroadcastStateTransitionResponse {}) }),
            Attempt::Stall => Box::pin(futures::future::pending()),
        }
    }
}

/// Exercises production DAPI and SDK retry loops with only the transport replaced.
pub(in crate::platform::transition) async fn retrying_broadcast(
    transition: &StateTransition,
    attempts: Vec<Attempt>,
    retries: usize,
) -> (Result<(), Error>, Vec<Vec<u8>>, usize) {
    let addresses =
        "http://127.0.0.1:3001,http://127.0.0.1:3002,http://127.0.0.1:3003,http://127.0.0.1:3004"
            .parse::<AddressList>()
            .unwrap();
    let client = DapiClient::new(addresses.clone(), RequestSettings::default());
    let attempts = Arc::new(Mutex::new(VecDeque::from(attempts)));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut executions = 0;
    let outcome = broadcast_with_retries(
        transition,
        &addresses,
        RequestSettings {
            retries: Some(retries),
            timeout: Some(Duration::from_millis(1)),
            ..Default::default()
        },
        |request, settings| {
            executions += 1;
            let request = MemoryBroadcast {
                request,
                attempts: Arc::clone(&attempts),
                sent: Arc::clone(&sent),
            };
            let client = &client;
            async move {
                client
                    .execute(request, settings)
                    .await
                    .map_err(|error| error.inner_into())
            }
        },
        PlatformVersion::latest(),
    )
    .await;
    assert!(
        attempts.lock().unwrap().is_empty(),
        "all expected sends happened"
    );
    let sent = sent.lock().unwrap().clone();
    let expected = transition.broadcast_request_for_state_transition().unwrap();
    assert!(sent.iter().all(|bytes| bytes == &expected.state_transition));
    (finish_broadcast(outcome), sent, executions)
}

fn signed_transition() -> StateTransition {
    StateTransition::IdentityCreate(
        IdentityCreateTransitionV0 {
            signature: vec![7; 65].into(),
            ..Default::default()
        }
        .into(),
    )
}

fn refusal() -> Attempt {
    Attempt::Error(Status::resource_exhausted("mempool is full"))
}

#[tokio::test(start_paused = true)]
async fn should_preserve_earlier_ambiguity_through_a_final_transport_refusal() {
    for ambiguous in [
        Attempt::Error(Status::unavailable("acknowledgement lost")),
        Attempt::Error(Status::resource_exhausted(
            "broadcasting state transition is timed out",
        )),
        Attempt::Stall,
    ] {
        let (outcome, sent, executions) =
            retrying_broadcast(&signed_transition(), vec![ambiguous, refusal()], 1).await;
        // Keep the existing final broadcast-only error, but wait on the exact
        // signed transition because an earlier node may have accepted it.
        assert!(outcome
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("mempool is full"));
        assert_eq!(sent.len(), 2);
        assert_eq!(executions, 1, "one inner retrying DAPI execution");
        assert_eq!(
            wait_after_broadcast(outcome, async { Ok(42) })
                .await
                .unwrap(),
            42
        );
    }
}

#[tokio::test(start_paused = true)]
async fn should_not_wait_when_every_transport_attempt_definitely_refused() {
    let (outcome, sent, executions) =
        retrying_broadcast(&signed_transition(), vec![refusal(), refusal()], 1).await;
    assert_eq!(sent.len(), 2);
    assert_eq!(executions, 1);
    let final_error = outcome.as_ref().unwrap_err().to_string();
    let result: Result<(), Error> = wait_after_broadcast(outcome, async {
        panic!("definite refusals must not poll a wait")
    })
    .await;
    assert_eq!(result.unwrap_err().to_string(), final_error);
}

#[tokio::test(start_paused = true)]
async fn should_keep_eventual_success_and_verified_wait_failures_unchanged() {
    let (success, sent, _) = retrying_broadcast(
        &signed_transition(),
        vec![
            Attempt::Error(Status::unavailable("lost")),
            Attempt::Success,
        ],
        1,
    )
    .await;
    assert!(success.is_ok());
    assert_eq!(sent.len(), 2);
    assert_eq!(
        wait_after_broadcast(success, async { Ok(42) })
            .await
            .unwrap(),
        42
    );

    let (ambiguous, _, _) = retrying_broadcast(
        &signed_transition(),
        vec![Attempt::Error(Status::unavailable("lost")), refusal()],
        1,
    )
    .await;
    let result: Result<(), Error> = wait_after_broadcast(ambiguous, async {
        Err(Error::InvalidProvedResponse("proof rejected".into()))
    })
    .await;
    assert!(
        matches!(result, Err(Error::StateTransitionResultUnconfirmed(cause)) if matches!(*cause, Error::InvalidProvedResponse(_)))
    );
}

#[tokio::test(start_paused = true)]
async fn should_keep_concurrent_broadcast_observations_separate() {
    let transition = signed_transition();
    let (ambiguous, refused) = futures::join!(
        retrying_broadcast(
            &transition,
            vec![Attempt::Error(Status::unavailable("lost")), refusal()],
            1
        ),
        retrying_broadcast(&transition, vec![refusal(), refusal()], 1),
    );
    assert!(ambiguous
        .0
        .as_ref()
        .unwrap_err()
        .is_state_transition_result_unconfirmed());
    assert!(!refused
        .0
        .as_ref()
        .unwrap_err()
        .is_state_transition_result_unconfirmed());
}

fn consensus_refusal() -> Status {
    let consensus: ConsensusError = IdentityNotFoundError::new(Identifier::from([1; 32])).into();
    let bytes = consensus
        .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
        .unwrap();
    let mut status = Status::invalid_argument("owner unavailable on this node");
    status.metadata_mut().insert_bin(
        "dash-serialized-consensus-error-bin",
        MetadataValue::from_bytes(&bytes),
    );
    status
}

#[tokio::test(start_paused = true)]
async fn should_return_consensus_rejection_after_connection_establishment_fails() {
    // tonic wraps failures from its connector before mapping them to Unavailable.
    // Preserve that source through real inner retries and SDK error conversion.
    let connect_error = Status::from_error(Box::new(ConnectError(Box::new(io::Error::new(
        io::ErrorKind::ConnectionRefused,
        "connection refused",
    )))));
    assert_eq!(connect_error.code(), Code::Unavailable);
    let (outcome, sent, executions) = retrying_broadcast(
        &signed_transition(),
        vec![
            Attempt::Error(connect_error),
            Attempt::Error(consensus_refusal()),
        ],
        1,
    )
    .await;
    assert_eq!(sent.len(), 2);
    assert_eq!(executions, 1);
    let result: Result<(), Error> = wait_after_broadcast(outcome, async {
        panic!("a connection failure never submitted the first request")
    })
    .await;
    assert!(matches!(
        result,
        Err(Error::Protocol(ProtocolError::ConsensusError(error)))
            if matches!(*error, ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(_)))
    ));
}

#[tokio::test(start_paused = true)]
async fn should_keep_real_ambiguity_despite_a_later_state_dependent_consensus_rejection() {
    let (outcome, sent, executions) = retrying_broadcast(
        &signed_transition(),
        vec![
            Attempt::Error(Status::unavailable("acknowledgement lost")),
            Attempt::Error(consensus_refusal()),
        ],
        1,
    )
    .await;
    assert_eq!(sent.len(), 2);
    assert_eq!(executions, 1);
    assert_eq!(
        wait_after_broadcast(outcome, async { Ok(42) })
            .await
            .unwrap(),
        42
    );
}

#[tokio::test(start_paused = true)]
async fn should_treat_exact_dapi_processing_rejection_as_local_to_its_attempt() {
    for rejection in [
        DAPI_PROCESSING_REJECTION.to_owned(),
        format!("Internal error: {DAPI_PROCESSING_REJECTION}"),
    ] {
        let (refused, sent, executions) = retrying_broadcast(
            &signed_transition(),
            vec![Attempt::Error(Status::internal(&rejection))],
            1,
        )
        .await;
        assert_eq!(sent.len(), 1);
        assert_eq!(executions, 1);
        let result: Result<(), Error> = wait_after_broadcast(refused, async {
            panic!("both attempts returned the exact local processing refusal")
        })
        .await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(DAPI_PROCESSING_REJECTION));

        for ambiguous in [
            Status::unavailable("acknowledgement lost"),
            Status::internal("Failed checking state transition: connection reset"),
        ] {
            let (outcome, sent, executions) = retrying_broadcast(
                &signed_transition(),
                vec![
                    Attempt::Error(ambiguous),
                    Attempt::Error(Status::internal(&rejection)),
                ],
                1,
            )
            .await;
            assert_eq!(sent.len(), 2);
            assert_eq!(executions, 1);
            assert_eq!(
                wait_after_broadcast(outcome, async { Ok(42) })
                    .await
                    .unwrap(),
                42
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn should_retry_raw_cache_but_stop_on_confirmed_presence() {
    let (outcome, sent, _) = retrying_broadcast(
        &signed_transition(),
        vec![
            Attempt::Error(Status::already_exists("tx already exists in cache")),
            Attempt::Success,
        ],
        1,
    )
    .await;
    assert!(outcome.is_ok());
    assert_eq!(sent.len(), 2);
    for message in [
        "state transition already in mempool",
        "state transition already in chain",
    ] {
        let (outcome, sent, _) = retrying_broadcast(
            &signed_transition(),
            vec![Attempt::Error(Status::already_exists(message))],
            3,
        )
        .await;
        assert_eq!(sent.len(), 1);
        assert!(outcome
            .as_ref()
            .unwrap_err()
            .is_state_transition_result_unconfirmed());
        assert_eq!(
            wait_after_broadcast(outcome, async { Ok(7) })
                .await
                .unwrap(),
            7
        );
    }
}

#[tokio::test(start_paused = true)]
async fn should_prioritize_structured_refusal_without_erasing_earlier_delivery() {
    for earlier in [false, true] {
        let refusal = consensus_refusal();
        let mut unusual = Status::unknown("Drive refused this attempt");
        *unusual.metadata_mut() = refusal.metadata().clone();
        let mut attempts = vec![];
        if earlier {
            attempts.push(Attempt::Error(Status::unavailable("lost")));
        }
        attempts.push(Attempt::Error(unusual));
        let (outcome, sent, _) = retrying_broadcast(&signed_transition(), attempts, 3).await;
        assert_eq!(sent.len(), if earlier { 2 } else { 1 });
        assert_eq!(
            outcome
                .as_ref()
                .unwrap_err()
                .is_state_transition_result_unconfirmed(),
            earlier
        );
        if earlier {
            assert_eq!(
                wait_after_broadcast(outcome, async { Ok(9) })
                    .await
                    .unwrap(),
                9
            );
        } else {
            assert!(matches!(
                outcome,
                Err(Error::Protocol(ProtocolError::ConsensusError(_)))
            ));
        }
    }
}

#[tokio::test(start_paused = true)]
async fn should_retain_uncertainty_when_addresses_are_exhausted() {
    let (result, sent, _) = retrying_broadcast(
        &signed_transition(),
        (0..4)
            .map(|_| Attempt::Error(Status::unavailable("lost")))
            .collect(),
        6,
    )
    .await;
    assert_eq!(sent.len(), 4);
    let error = result.unwrap_err();
    assert!(error.is_state_transition_result_unconfirmed());
    assert!(error.to_string().contains("lost"));
}

#[test]
fn should_keep_read_classification_independent_from_broadcast() {
    let read = dapi_grpc::platform::v0::GetStatusRequest::default();
    for status in [
        Status::unavailable("lost"),
        Status::already_exists("tx already exists in cache"),
    ] {
        let error = TransportError::Grpc(status);
        let classification = read.classify_error(&error);
        assert!(!classification.possibly_accepted);
        assert_eq!(classification.retry, error.can_retry());
    }
}

#[tokio::test]
async fn should_distinguish_permanent_invalidity_from_state_dependent_refusal() {
    use dpp::consensus::basic::identity::DuplicatedIdentityPublicKeyIdBasicError;
    let permanent: ConsensusError = DuplicatedIdentityPublicKeyIdBasicError::new(vec![1, 1]).into();
    let error =
        Error::Protocol(ProtocolError::ConsensusError(Box::new(permanent))).into_unconfirmed();
    let result: Result<(), Error> = wait_after_broadcast(Err(error), async {
        panic!("intrinsic duplicate ids cannot execute")
    })
    .await;
    assert!(matches!(
        result,
        Err(Error::Protocol(ProtocolError::ConsensusError(_)))
    ));
    let stale: ConsensusError = IdentityNotFoundError::new(Identifier::from([1; 32])).into();
    let error = Error::Protocol(ProtocolError::ConsensusError(Box::new(stale))).into_unconfirmed();
    assert_eq!(
        wait_after_broadcast(Err(error), async { Ok(5) })
            .await
            .unwrap(),
        5
    );
}

#[tokio::test]
async fn should_keep_business_result_and_avoid_double_wrapping_unknown_result() {
    let result: Result<(), Error> = wait_after_broadcast(Ok(()), async {
        Err(Error::StateTransitionBroadcastError(
            StateTransitionBroadcastError {
                code: 20000,
                message: "owner missing".into(),
                cause: Some(IdentityNotFoundError::new(Identifier::from([1; 32])).into()),
            },
        ))
    })
    .await;
    assert!(matches!(
        result,
        Err(Error::StateTransitionBroadcastError(_))
    ));
    let result: Result<(), Error> = wait_after_broadcast(Ok(()), async {
        Err(Error::InvalidProvedResponse("bad proof".into()).into_unconfirmed())
    })
    .await;
    assert!(
        matches!(result, Err(Error::StateTransitionResultUnconfirmed(cause)) if matches!(*cause, Error::InvalidProvedResponse(_)))
    );
}
