//! Advisory protocol observations. These never authorize or reject operations.

use super::Sdk;
use crate::platform::Fetch;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::version::PlatformVersion;
use futures::future::{select, Either};
use rs_dapi_client::transport::sleep;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

const CHECK_BUDGET: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolCompatibilityUnavailableReason {
    NotChecked,
    TimedOut,
    CheckFailed,
    ProofsDisabled,
}

/// Outcome of the most recent explicit check, independent of past observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolCompatibilityState {
    Unavailable {
        reason: ProtocolCompatibilityUnavailableReason,
    },
    Checking,
    Checked,
}

/// Authenticated network evidence, not a promise that any operation is supported.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProtocolCompatibility {
    pub generation: u64,
    pub state: ProtocolCompatibilityState,
    pub verified_protocol_version: Option<u32>,
    pub effective_protocol_version: u32,
    pub maximum_supported_protocol_version: u32,
}

impl ProtocolCompatibility {
    /// Whether authenticated evidence exceeds this SDK's supported tables.
    /// This informational predicate is independent of the latest check status
    /// and never authorizes or denies a wallet operation.
    pub fn newer_protocol_available(&self) -> bool {
        self.verified_protocol_version
            .is_some_and(|version| version > self.maximum_supported_protocol_version)
    }
}

#[derive(Debug)]
pub(super) struct Compatibility {
    generation: u64,
    state: ProtocolCompatibilityState,
    verified_version: Option<u32>,
    attempt_verified: bool,
    deadline: Option<Instant>,
}

impl Compatibility {
    pub(super) fn new() -> Mutex<Self> {
        Mutex::new(Self {
            generation: 0,
            state: ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::NotChecked,
            },
            verified_version: None,
            attempt_verified: false,
            deadline: None,
        })
    }

    fn expire(&mut self) {
        if self.state == ProtocolCompatibilityState::Checking
            && self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.state = ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::TimedOut,
            };
            self.deadline = None;
        }
    }
}

impl Sdk {
    fn compatibility_lock(&self) -> MutexGuard<'_, Compatibility> {
        let mut state = self
            .protocol_compatibility
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.expire();
        state
    }

    /// Start a bounded informational check; does not invalidate prepared work.
    pub fn begin_protocol_compatibility_check(&self) -> u64 {
        let mut state = self.compatibility_lock();
        state.generation = state.generation.saturating_add(1);
        state.state = ProtocolCompatibilityState::Checking;
        state.attempt_verified = false;
        state.deadline = Some(Instant::now() + CHECK_BUDGET);
        state.generation
    }

    pub fn protocol_compatibility(&self) -> ProtocolCompatibility {
        let state = self.compatibility_lock();
        ProtocolCompatibility {
            generation: state.generation,
            state: state.state,
            verified_protocol_version: state.verified_version,
            effective_protocol_version: self.protocol_version_number(),
            maximum_supported_protocol_version: PlatformVersion::latest().protocol_version,
        }
    }

    /// One proven epoch query using existing transport retries. Its proof,
    /// signature and metadata freshness must pass the ordinary verifier.
    pub async fn refresh_protocol_compatibility(&self, generation: u64) -> ProtocolCompatibility {
        let deadline = {
            let state = self.compatibility_lock();
            if state.generation != generation || state.state != ProtocolCompatibilityState::Checking
            {
                drop(state);
                return self.protocol_compatibility();
            }
            state.deadline
        };
        let Some(deadline) = deadline else {
            return self.protocol_compatibility();
        };
        let mut checking_sdk = self.clone();
        checking_sdk.compatibility_check_generation = Some(generation);
        let result = if !self.prove() {
            Err(ProtocolCompatibilityUnavailableReason::ProofsDisabled)
        } else {
            // The transport owns the executor-specific timer: Tokio natively,
            // a JavaScript timer in browsers. Poll the timer first so expiry
            // wins when the query is also ready at the budget boundary.
            let timeout = sleep(deadline.saturating_duration_since(Instant::now()));
            let query = ExtendedEpochInfo::fetch_with_metadata(&checking_sdk, 0u16, None);
            futures::pin_mut!(timeout, query);
            match select(timeout, query).await {
                Either::Right((Ok(_), _)) => Ok(()),
                Either::Right((Err(error), _)) => {
                    tracing::warn!(%error, generation, network = ?self.network, "protocol observation failed");
                    Err(ProtocolCompatibilityUnavailableReason::CheckFailed)
                }
                Either::Left(_) => Err(ProtocolCompatibilityUnavailableReason::TimedOut),
            }
        };
        {
            let mut state = self.compatibility_lock();
            if state.generation == generation && state.state == ProtocolCompatibilityState::Checking
            {
                state.deadline = None;
                state.state = match result {
                    Ok(()) if state.attempt_verified => ProtocolCompatibilityState::Checked,
                    Ok(()) => ProtocolCompatibilityState::Unavailable {
                        reason: ProtocolCompatibilityUnavailableReason::CheckFailed,
                    },
                    Err(reason) => ProtocolCompatibilityState::Unavailable { reason },
                };
            }
        }
        let snapshot = self.protocol_compatibility();
        tracing::info!(generation, network = ?self.network, state = ?snapshot.state,
            observed_version = ?snapshot.verified_protocol_version,
            effective_version = snapshot.effective_protocol_version,
            elapsed_ms = deadline.checked_sub(CHECK_BUDGET).map(|start| start.elapsed().as_millis()),
            "protocol observation completed");
        snapshot
    }

    /// Called only after proof, quorum signature and freshness verification.
    pub(super) fn observe_compatible_metadata(&self, version: u32) {
        let mut state = self.compatibility_lock();
        if let Some(generation) = self.compatibility_check_generation {
            if state.generation == generation && state.state == ProtocolCompatibilityState::Checking
            {
                state.attempt_verified = true;
            }
        }
        // A superseded query still authenticates network evidence for this SDK
        // instance. Only its attempt state is stale, not the verified metadata.
        state.verified_version = Some(
            state
                .verified_version
                .map_or(version, |old| old.max(version)),
        );
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::platform::transition::broadcast::BroadcastStateTransition;
    use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
    use crate::platform::Query;
    use crate::sdk::SdkInstance;
    use crate::SdkBuilder;
    use dapi_grpc::platform::v0::BroadcastStateTransitionResponse;
    use dapi_grpc::platform::v0::{GetEpochsInfoRequest, ResponseMetadata};
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::StateTransition;
    use rs_dapi_client::ExecutionResponse;

    fn sdk() -> Sdk {
        SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(13).expect("seed"))
            .with_height_tolerance(None)
            .build()
            .expect("SDK")
    }
    async fn expect_check(sdk: &mut Sdk) {
        sdk.mock()
            .expect_fetch::<ExtendedEpochInfo, _>(
                0u16,
                Some(ExtendedEpochInfo::from(ExtendedEpochInfoV0 {
                    index: 0,
                    first_block_time: 0,
                    first_block_height: 0,
                    first_core_block_height: 0,
                    fee_multiplier_permille: 0,
                    protocol_version: PlatformVersion::latest().protocol_version,
                })),
            )
            .await
            .expect("one proven genesis query");
    }

    fn verified(sdk: &Sdk, version: u32) {
        sdk.accept_verified_metadata(
            "get_epochs_info",
            &ResponseMetadata {
                protocol_version: version,
                ..Default::default()
            },
            None,
        )
        .expect("fresh authenticated test metadata");
    }

    #[test]
    fn should_classify_newer_evidence_independently_of_check_status() {
        let maximum = PlatformVersion::latest().protocol_version;
        for state in [
            ProtocolCompatibilityState::Checking,
            ProtocolCompatibilityState::Checked,
            ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::CheckFailed,
            },
        ] {
            for (version, expected) in [
                (None, false),
                (Some(maximum - 1), false),
                (Some(maximum), false),
                (Some(maximum + 1), true),
            ] {
                let snapshot = ProtocolCompatibility {
                    generation: 0,
                    state,
                    verified_protocol_version: version,
                    effective_protocol_version: maximum,
                    maximum_supported_protocol_version: maximum,
                };
                assert_eq!(snapshot.newer_protocol_available(), expected);
            }
        }
    }

    #[tokio::test]
    async fn should_skip_proofs_disabled_checks_without_losing_past_evidence() {
        for evidence in [None, Some(PlatformVersion::latest().protocol_version + 1)] {
            let mut sdk = SdkBuilder::new_mock()
                .with_proofs(false)
                .with_height_tolerance(None)
                .build()
                .unwrap();
            if let Some(version) = evidence {
                verified(&sdk, version);
            }
            let generation = sdk.begin_protocol_compatibility_check();
            let result = sdk.refresh_protocol_compatibility(generation).await;
            assert_eq!(
                result.state,
                ProtocolCompatibilityState::Unavailable {
                    reason: ProtocolCompatibilityUnavailableReason::ProofsDisabled,
                }
            );
            assert_eq!(result.verified_protocol_version, evidence);
            assert_eq!(result.newer_protocol_available(), evidence.is_some());
            // Epochs have no unproved wire query. Count the proven request
            // that the lifecycle must have skipped before query encoding.
            let mut settings = sdk.query_settings();
            settings.prove = true;
            let request: GetEpochsInfoRequest = 0u16.query(&settings).unwrap();
            assert_eq!(sdk.mock().request_count(&request).await, 0);
        }
    }

    #[tokio::test]
    async fn should_recover_after_failed_check_only_by_successful_explicit_retry() {
        let mut sdk = sdk();
        let first = sdk.begin_protocol_compatibility_check();
        sdk.refresh_protocol_compatibility(first).await;
        expect_check(&mut sdk).await;
        let retry = sdk.begin_protocol_compatibility_check();
        assert!(retry > first);
        assert_eq!(
            sdk.refresh_protocol_compatibility(retry).await.state,
            ProtocolCompatibilityState::Checked
        );
    }

    #[tokio::test]
    async fn should_timeout_while_a_query_is_pending_and_cancel_that_query() {
        let sdk = sdk();
        let SdkInstance::Mock { dapi, .. } = &sdk.inner else {
            panic!("mock")
        };
        let _blocked_transport = dapi.lock().await;
        let generation = sdk.begin_protocol_compatibility_check();
        sdk.protocol_compatibility.lock().unwrap().deadline =
            Some(Instant::now() + Duration::from_millis(10));
        let result = sdk.refresh_protocol_compatibility(generation).await;
        assert!(matches!(
            result.state,
            ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::TimedOut
            }
        ));
        assert_eq!(result.verified_protocol_version, None);
    }

    #[tokio::test]
    async fn should_retain_superseded_evidence_without_overwriting_the_latest_check() {
        let mut sdk = sdk();
        let old = sdk.begin_protocol_compatibility_check();
        let mut old_query = sdk.clone();
        old_query.compatibility_check_generation = Some(old);
        let current = sdk.begin_protocol_compatibility_check();
        verified(&old_query, PlatformVersion::latest().protocol_version + 1);
        assert_eq!(
            sdk.protocol_compatibility().state,
            ProtocolCompatibilityState::Checking
        );
        assert_eq!(
            sdk.protocol_compatibility().verified_protocol_version,
            Some(PlatformVersion::latest().protocol_version + 1)
        );
        assert!(!sdk.protocol_compatibility.lock().unwrap().attempt_verified);
        assert_eq!(
            sdk.refresh_protocol_compatibility(old).await.generation,
            current
        );
        expect_check(&mut sdk).await;
        sdk.refresh_protocol_compatibility(current).await;
        verified(&old_query, 13);
        sdk.refresh_protocol_compatibility(old).await;
        assert_eq!(
            sdk.protocol_compatibility().state,
            ProtocolCompatibilityState::Checked
        );
    }

    #[tokio::test]
    async fn should_keep_late_evidence_when_the_latest_check_fails() {
        let sdk = sdk();
        let old = sdk.begin_protocol_compatibility_check();
        let mut old_query = sdk.clone();
        old_query.compatibility_check_generation = Some(old);
        let current = sdk.begin_protocol_compatibility_check();
        let failed = sdk.refresh_protocol_compatibility(current).await;
        assert_eq!(
            failed.state,
            ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::CheckFailed,
            }
        );
        verified(&old_query, PlatformVersion::latest().protocol_version + 1);
        let observed = sdk.protocol_compatibility();
        assert_eq!(observed.generation, current);
        assert_eq!(observed.state, failed.state);
        assert!(observed.newer_protocol_available());
        assert!(!sdk.protocol_compatibility.lock().unwrap().attempt_verified);
        assert_eq!(
            observed.effective_protocol_version,
            failed.effective_protocol_version
        );
        verified(&old_query, 13);
        assert_eq!(sdk.protocol_compatibility(), observed);
    }

    #[tokio::test]
    async fn should_query_once_and_share_observations_without_changing_preparations() {
        let mut sdk = sdk();
        let clone = sdk.clone();
        let version = sdk.version().protocol_version;
        expect_check(&mut sdk).await;
        let generation = sdk.begin_protocol_compatibility_check();
        assert_eq!(clone.version().protocol_version, version);
        assert_eq!(
            clone.protocol_compatibility().state,
            ProtocolCompatibilityState::Checking
        );
        let result = sdk.refresh_protocol_compatibility(generation).await;
        assert_eq!(result.state, ProtocolCompatibilityState::Checked);
        assert_eq!(clone.protocol_compatibility(), result);
        let request: GetEpochsInfoRequest = 0u16.query(&sdk.query_settings()).unwrap();
        assert_eq!(sdk.mock().request_count(&request).await, 1);
    }

    #[tokio::test]
    async fn should_keep_newer_evidence_after_failed_checks_and_lower_responses() {
        let sdk = sdk();
        let version = sdk.version().protocol_version;
        let future_version = PlatformVersion::latest().protocol_version + 1;
        verified(&sdk, future_version);
        let generation = sdk.begin_protocol_compatibility_check();
        let result = sdk.refresh_protocol_compatibility(generation).await;
        assert!(matches!(
            result.state,
            ProtocolCompatibilityState::Unavailable { .. }
        ));
        verified(&sdk, version);
        assert_eq!(
            sdk.protocol_compatibility().verified_protocol_version,
            Some(future_version)
        );
        assert_eq!(sdk.version().protocol_version, version);
    }

    #[test]
    fn should_not_trust_plain_or_stale_metadata_as_network_evidence() {
        let sdk = sdk();
        sdk.verify_response_metadata(
            "get_epochs_info",
            &ResponseMetadata {
                protocol_version: PlatformVersion::latest().protocol_version + 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(sdk.protocol_compatibility().verified_protocol_version, None);
        let fresh_sdk = SdkBuilder::new_mock()
            .with_time_tolerance(Some(1000))
            .build()
            .unwrap();
        assert!(fresh_sdk
            .accept_verified_metadata(
                "get_epochs_info",
                &ResponseMetadata {
                    protocol_version: PlatformVersion::latest().protocol_version + 1,
                    time_ms: 1,
                    ..Default::default()
                },
                None
            )
            .is_err());
        assert_eq!(
            fresh_sdk.protocol_compatibility().verified_protocol_version,
            None
        );
    }

    #[tokio::test]
    async fn should_expire_queue_budget_without_erasing_verified_evidence() {
        let sdk = sdk();
        verified(&sdk, 13);
        let generation = sdk.begin_protocol_compatibility_check();
        sdk.protocol_compatibility.lock().unwrap().deadline =
            Some(Instant::now() - Duration::from_secs(1));
        let result = sdk.refresh_protocol_compatibility(generation).await;
        assert_eq!(
            result.state,
            ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::TimedOut
            }
        );
        assert_eq!(result.verified_protocol_version, Some(13));
        assert_eq!(sdk.version().protocol_version, 13);
    }
    #[tokio::test]
    async fn should_never_gate_the_real_broadcast_path_on_informational_checks() {
        for phase in 0..4 {
            let sdk = SdkBuilder::new_mock()
                .with_height_tolerance(None)
                .build()
                .unwrap();
            if phase > 0 {
                let attempt = sdk.begin_protocol_compatibility_check();
                if phase > 1 {
                    sdk.refresh_protocol_compatibility(attempt).await;
                }
            }
            if phase == 3 {
                sdk.accept_verified_metadata(
                    "get_epochs_info",
                    &ResponseMetadata {
                        protocol_version: PlatformVersion::latest().protocol_version + 1,
                        ..Default::default()
                    },
                    None,
                )
                .unwrap();
            }
            let transition = StateTransition::Batch(BatchTransitionV0::default().into());
            let request = transition.broadcast_request_for_state_transition().unwrap();
            let SdkInstance::Mock { dapi, .. } = &sdk.inner else {
                panic!("mock")
            };
            dapi.lock()
                .await
                .expect(
                    &request,
                    &Ok(ExecutionResponse {
                        inner: BroadcastStateTransitionResponse::default(),
                        retries: 0,
                        address: "http://127.0.0.1".parse().unwrap(),
                    }),
                )
                .unwrap();
            transition
                .broadcast(&sdk, None)
                .await
                .expect("advisory state never blocks submission");
            assert_eq!(dapi.lock().await.request_count(&request), 1);
        }
    }
}
