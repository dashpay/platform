//! Real DAPI and SDK retry loops over an in-memory transport, without sockets.

use super::*;
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::metadata::MetadataValue;
use dapi_grpc::tonic::Status;
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportRequest,
};
use rs_dapi_client::{ConnectionPool, DapiClient, Uri};
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::Duration;

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

/// Uses the same observed broadcast helper as production, including both retry
/// layers. Only the transport itself is replaced with deterministic responses.
pub(in crate::platform::transition) async fn retrying_broadcast(
    transition: &StateTransition,
    attempts: Vec<Attempt>,
    retries: usize,
) -> (BroadcastOutcome, Vec<Vec<u8>>, usize) {
    retrying_broadcast_with_policy(transition, attempts, retries, BroadcastPolicy::Standard).await
}

async fn retrying_broadcast_with_policy(
    transition: &StateTransition,
    attempts: Vec<Attempt>,
    retries: usize,
    policy: BroadcastPolicy,
) -> (BroadcastOutcome, Vec<Vec<u8>>, usize) {
    let addresses =
        "http://127.0.0.1:3001,http://127.0.0.1:3002,http://127.0.0.1:3003,http://127.0.0.1:3004"
            .parse::<AddressList>()
            .unwrap();
    let client = DapiClient::new(addresses.clone(), RequestSettings::default());
    let attempts = Arc::new(Mutex::new(VecDeque::from(attempts)));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut executions = 0;
    let outcome = broadcast_and_observe(
        transition,
        &addresses,
        RequestSettings {
            retries: Some(retries),
            timeout: Some(Duration::from_millis(1)),
            ..Default::default()
        },
        policy,
        |request, settings, observation| {
            executions += 1;
            let request = MemoryBroadcast {
                request,
                attempts: Arc::clone(&attempts),
                sent: Arc::clone(&sent),
            };
            let client = &client;
            async move {
                client
                    .execute_with_transport_error_observer(request, settings, &|error| {
                        if transport_broadcast_may_have_been_accepted(error) {
                            observation.store(true, Ordering::Relaxed);
                        }
                    })
                    .await
                    .map_err(|error| error.inner_into())
            }
        },
    )
    .await;
    assert!(
        attempts.lock().unwrap().is_empty(),
        "all expected sends happened"
    );
    let sent = sent.lock().unwrap().clone();
    let expected = transition.broadcast_request_for_state_transition().unwrap();
    assert!(sent.iter().all(|bytes| bytes == &expected.state_transition));
    (outcome, sent, executions)
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

#[test_case::test_case(BroadcastPolicy::Standard; "standard")]
#[test_case::test_case(BroadcastPolicy::TransportOnly; "transport_only")]
#[tokio::test(start_paused = true)]
async fn should_preserve_earlier_ambiguity_through_a_final_transport_refusal(
    policy: BroadcastPolicy,
) {
    for ambiguous in [
        Attempt::Error(Status::unavailable("acknowledgement lost")),
        Attempt::Error(Status::resource_exhausted(
            "broadcasting state transition is timed out",
        )),
        Attempt::Stall,
    ] {
        let (outcome, sent, executions) = retrying_broadcast_with_policy(
            &signed_transition(),
            vec![ambiguous, refusal()],
            1,
            policy,
        )
        .await;
        // Keep the existing final broadcast-only error, but wait on the exact
        // signed transition because an earlier node may have accepted it.
        assert!(outcome
            .result
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

#[test_case::test_case(BroadcastPolicy::Standard; "standard")]
#[test_case::test_case(BroadcastPolicy::TransportOnly; "transport_only")]
#[tokio::test(start_paused = true)]
async fn should_not_wait_when_every_transport_attempt_definitely_refused(policy: BroadcastPolicy) {
    let (outcome, sent, executions) =
        retrying_broadcast_with_policy(&signed_transition(), vec![refusal(), refusal()], 1, policy)
            .await;
    assert_eq!(sent.len(), 2);
    assert_eq!(executions, 1);
    let final_error = outcome.result.as_ref().unwrap_err().to_string();
    let result: Result<(), Error> = wait_after_broadcast(outcome, async {
        panic!("definite refusals must not poll a wait")
    })
    .await;
    assert_eq!(result.unwrap_err().to_string(), final_error);
}

#[test_case::test_case(BroadcastPolicy::Standard; "standard")]
#[test_case::test_case(BroadcastPolicy::TransportOnly; "transport_only")]
#[tokio::test(start_paused = true)]
async fn should_keep_eventual_success_and_verified_wait_failures_unchanged(
    policy: BroadcastPolicy,
) {
    let (success, sent, _) = retrying_broadcast_with_policy(
        &signed_transition(),
        vec![
            Attempt::Error(Status::unavailable("lost")),
            Attempt::Success,
        ],
        1,
        policy,
    )
    .await;
    assert!(success.result.is_ok());
    assert_eq!(sent.len(), 2);
    assert_eq!(
        wait_after_broadcast(success, async { Ok(42) })
            .await
            .unwrap(),
        42
    );

    let (ambiguous, _, _) = retrying_broadcast_with_policy(
        &signed_transition(),
        vec![Attempt::Error(Status::unavailable("lost")), refusal()],
        1,
        policy,
    )
    .await;
    let result: Result<(), Error> = wait_after_broadcast(ambiguous, async {
        Err(Error::InvalidProvedResponse("proof rejected".into()))
    })
    .await;
    assert!(matches!(result, Err(Error::InvalidProvedResponse(_))));
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
    assert!(ambiguous.0.possibly_accepted);
    assert!(!refused.0.possibly_accepted);
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
            vec![
                Attempt::Error(Status::internal(&rejection)),
                Attempt::Error(Status::internal(&rejection)),
            ],
            1,
        )
        .await;
        assert_eq!(sent.len(), 2);
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
