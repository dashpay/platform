//! Real DAPI and SDK retry loops over an in-memory transport, without sockets.

use super::*;
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::Status;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportRequest,
};
use rs_dapi_client::{ConnectionPool, DapiClient, Uri};
use std::collections::VecDeque;
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
    let addresses =
        "http://127.0.0.1:3001,http://127.0.0.1:3002,http://127.0.0.1:3003,http://127.0.0.1:3004"
            .parse::<AddressList>()
            .unwrap();
    let client = DapiClient::new(addresses.clone(), RequestSettings::default());
    let attempts = Arc::new(Mutex::new(VecDeque::from(attempts)));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut executions = 0;
    let outcome = broadcast_and_observe_with_retries(
        transition,
        &addresses,
        RequestSettings {
            retries: Some(retries),
            timeout: Some(Duration::from_millis(1)),
            ..Default::default()
        },
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

#[tokio::test(start_paused = true)]
async fn should_not_wait_when_every_transport_attempt_definitely_refused() {
    let (outcome, sent, executions) =
        retrying_broadcast(&signed_transition(), vec![refusal(), refusal()], 1).await;
    assert_eq!(sent.len(), 2);
    assert_eq!(executions, 1);
    let final_error = outcome.result.as_ref().unwrap_err().to_string();
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
    assert!(success.result.is_ok());
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
