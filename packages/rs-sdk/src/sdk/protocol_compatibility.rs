//! Opt-in, proof-authenticated readiness for Platform writes.
//!
//! This is client policy only. It does not change consensus, legacy version
//! discovery, or read availability. The trust boundary is
//! `Sdk::accept_verified_metadata`, never the public metadata freshness helper.

use super::Sdk;
use crate::platform::Fetch;
use crate::Error;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::version::PlatformVersion;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

const CHECK_BUDGET: Duration = Duration::from_secs(20);
static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

tokio::task_local! {
    // A stack keeps an inner operation from replacing its caller's confirmed
    // generation with a newly captured permit. Each future has its own scope.
    static EXPECTED_WRITE_PERMITS: Vec<PlatformWritePermit>;
}

/// Execute queued work under the permit captured when the user confirmed it.
/// Every strict preparation and broadcast guard validates this permit, even
/// when the operation's SDK helpers capture their own context after queueing.
/// Nested scopes must satisfy every enclosing permit and restore their caller
/// on completion or cancellation. Spawned tasks do not inherit task-local state:
/// pass the original permit and scope the future that the worker actually polls.
pub async fn with_platform_write_permit<F: Future>(
    permit: PlatformWritePermit,
    future: F,
) -> F::Output {
    with_platform_write_permits(vec![permit], future).await
}

/// Restore a captured stack of confirmation permits on a worker's future.
/// Existing enclosing permits remain in force, including when native queues
/// carry nested caller scopes across threads.
pub async fn with_platform_write_permits<F: Future>(
    permits: Vec<PlatformWritePermit>,
    future: F,
) -> F::Output {
    let mut enclosing = EXPECTED_WRITE_PERMITS
        .try_with(Clone::clone)
        .unwrap_or_default();
    enclosing.extend(permits);
    EXPECTED_WRITE_PERMITS.scope(enclosing, future).await
}

/// Existing clients retain their previous behavior until they opt in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProtocolCompatibilityPolicy {
    #[default]
    Legacy,
    RequireVerified,
}

/// A failed check never implies that an application update is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolCompatibilityUnavailableReason {
    NotChecked,
    TimedOut,
    CheckFailed,
    ProofsDisabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolCompatibilityState {
    Unavailable {
        reason: ProtocolCompatibilityUnavailableReason,
    },
    Checking,
    Supported,
    Unsupported,
}

/// A coherent, clone-shared compatibility snapshot. Observed versions may be
/// newer than this binary understands; the construction version never is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProtocolCompatibility {
    pub generation: u64,
    pub state: ProtocolCompatibilityState,
    pub verified_protocol_version: Option<u32>,
    pub effective_protocol_version: u32,
    pub maximum_supported_protocol_version: u32,
}

/// Capture before preparation, validate again immediately before spending.
/// Instance identity also rejects a preparation from a replaced runtime on
/// the same network. These fields are transport data, not authorization:
/// validation always checks current readiness first.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlatformWritePermit {
    pub instance_id: u64,
    pub generation: u64,
    pub protocol_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolCompatibilityError {
    #[error("Platform protocol compatibility has not been verified; retry the check")]
    Unavailable,
    #[error("Platform protocol compatibility is being checked")]
    Checking,
    #[error("network protocol {network_version} is not supported (maximum supported: {maximum_supported_version}); a newer application is required")]
    Unsupported {
        network_version: u32,
        maximum_supported_version: u32,
    },
    #[error(
        "Platform compatibility changed after preparation; prepare and confirm the operation again"
    )]
    PreparationExpired,
}

#[derive(Debug)]
pub(super) struct Compatibility {
    instance_id: u64,
    policy: ProtocolCompatibilityPolicy,
    generation: u64,
    state: ProtocolCompatibilityState,
    verified_version: Option<u32>,
    effective_version: u32,
    deadline: Option<Instant>,
}

impl Compatibility {
    pub(super) fn new(effective_version: u32) -> Mutex<Self> {
        Mutex::new(Self {
            instance_id: NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            policy: ProtocolCompatibilityPolicy::Legacy,
            generation: 0,
            state: ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::NotChecked,
            },
            verified_version: None,
            effective_version,
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

    fn ensure(&self) -> Result<(), ProtocolCompatibilityError> {
        if self.policy == ProtocolCompatibilityPolicy::Legacy {
            return Ok(());
        }
        match self.state {
            ProtocolCompatibilityState::Supported => Ok(()),
            ProtocolCompatibilityState::Checking => Err(ProtocolCompatibilityError::Checking),
            ProtocolCompatibilityState::Unavailable { .. } => {
                Err(ProtocolCompatibilityError::Unavailable)
            }
            ProtocolCompatibilityState::Unsupported => {
                Err(ProtocolCompatibilityError::Unsupported {
                    network_version: self.verified_version.unwrap_or_default(),
                    maximum_supported_version: PlatformVersion::latest().protocol_version,
                })
            }
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

    /// Changing policy invalidates prepared strict operations. Enabling strict
    /// policy requires an explicit successful check even if reads already ran.
    pub fn set_protocol_compatibility_policy(&self, policy: ProtocolCompatibilityPolicy) {
        let mut state = self.compatibility_lock();
        if state.policy == policy {
            return;
        }
        state.policy = policy;
        state.generation = state.generation.saturating_add(1);
        state.deadline = None;
        if state.state != ProtocolCompatibilityState::Unsupported {
            state.state = ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::NotChecked,
            };
        }
    }

    /// Blocks writes synchronously, before a worker can be queued. The complete
    /// check, including queue delay and quorum-key retrieval, has 20 seconds.
    pub fn begin_protocol_compatibility_check(&self) -> u64 {
        let mut state = self.compatibility_lock();
        state.generation = state.generation.saturating_add(1);
        if state.state != ProtocolCompatibilityState::Unsupported {
            state.state = ProtocolCompatibilityState::Checking;
            state.deadline = Some(Instant::now() + CHECK_BUDGET);
        }
        state.generation
    }

    pub(crate) fn requires_verified_protocol_compatibility(&self) -> bool {
        self.compatibility_lock().policy == ProtocolCompatibilityPolicy::RequireVerified
    }

    pub fn protocol_compatibility(&self) -> ProtocolCompatibility {
        let state = self.compatibility_lock();
        ProtocolCompatibility {
            generation: state.generation,
            state: state.state,
            verified_protocol_version: state.verified_version,
            effective_protocol_version: if state.policy == ProtocolCompatibilityPolicy::Legacy {
                self.protocol_version_number()
            } else {
                state.effective_version
            },
            maximum_supported_protocol_version: PlatformVersion::latest().protocol_version,
        }
    }

    /// Executes one proven genesis-epoch query (with the existing transport
    /// retry policy). The epoch payload is irrelevant; its signed metadata is
    /// authenticated by the normal proof verifier. Errors remain unavailable.
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
            match tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
                ExtendedEpochInfo::fetch_with_metadata(&checking_sdk, 0u16, None).await
            })
            .await
            {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(error)) => {
                    tracing::warn!(%error, generation, network = ?self.network, "protocol compatibility check failed");
                    Err(ProtocolCompatibilityUnavailableReason::CheckFailed)
                }
                Err(_) => Err(ProtocolCompatibilityUnavailableReason::TimedOut),
            }
        };
        {
            let mut state = self.compatibility_lock();
            if state.generation == generation && state.state == ProtocolCompatibilityState::Checking
            {
                state.deadline = None;
                state.state = match result {
                    Ok(()) if state.verified_version.is_some() => {
                        ProtocolCompatibilityState::Supported
                    }
                    Ok(()) => ProtocolCompatibilityState::Unavailable {
                        reason: ProtocolCompatibilityUnavailableReason::CheckFailed,
                    },
                    Err(reason) => ProtocolCompatibilityState::Unavailable { reason },
                };
            }
        }
        let snapshot = self.protocol_compatibility();
        tracing::info!(
            generation,
            network = ?self.network,
            state = ?snapshot.state,
            observed_version = ?snapshot.verified_protocol_version,
            effective_version = snapshot.effective_protocol_version,
            elapsed_ms = deadline.checked_sub(CHECK_BUDGET).map(|start| start.elapsed().as_millis()),
            "protocol compatibility check completed"
        );
        snapshot
    }

    /// Called exclusively after the ordinary proof/signature/freshness gate.
    pub(super) fn observe_compatible_metadata(&self, version: u32) {
        let mut state = self.compatibility_lock();
        if let Some(generation) = self.compatibility_check_generation {
            if state.generation != generation || state.state != ProtocolCompatibilityState::Checking
            {
                return;
            }
        }
        state.verified_version = Some(
            state
                .verified_version
                .map_or(version, |old| old.max(version)),
        );
        if version > PlatformVersion::latest().protocol_version
            || PlatformVersion::get(version).is_err()
        {
            state.state = ProtocolCompatibilityState::Unsupported;
            state.deadline = None;
        } else {
            // Ordinary reads cannot clear failed or in-progress explicit checks.
            state.effective_version = state.effective_version.max(version);
        }
    }

    pub fn ensure_platform_write_compatible(&self) -> Result<(), Error> {
        let state = self.compatibility_lock();
        self.ensure_preparation(&state)
    }

    fn ensure_preparation(&self, state: &Compatibility) -> Result<(), Error> {
        state.ensure()?;
        if state.policy == ProtocolCompatibilityPolicy::RequireVerified {
            let is_expired = |permit: &PlatformWritePermit| {
                state.instance_id != permit.instance_id
                    || state.generation != permit.generation
                    || state.effective_version != permit.protocol_version
            };
            let scoped_permit_expired = EXPECTED_WRITE_PERMITS
                .try_with(|permits| permits.iter().any(is_expired))
                .unwrap_or(false);
            if self.platform_preparation.as_ref().is_some_and(is_expired) || scoped_permit_expired {
                return Err(ProtocolCompatibilityError::PreparationExpired.into());
            }
        }
        Ok(())
    }

    /// Bind SDK write helpers to the generation before their first await. The
    /// clone shares readiness with its parent and carries the permit through
    /// nested helpers to the last pre-transport check.
    pub(crate) fn platform_write_context(&self) -> Result<Self, Error> {
        let permit = self.platform_write_permit()?;
        let mut sdk = self.clone();
        sdk.platform_preparation = Some(permit);
        Ok(sdk)
    }

    pub fn platform_version_for_write(&self) -> Result<&'static PlatformVersion, Error> {
        let state = self.compatibility_lock();
        self.ensure_preparation(&state)?;
        if state.policy == ProtocolCompatibilityPolicy::Legacy {
            Ok(self.version())
        } else {
            PlatformVersion::get(state.effective_version).map_err(Error::from)
        }
    }

    pub fn platform_write_permit(&self) -> Result<PlatformWritePermit, Error> {
        let state = self.compatibility_lock();
        self.ensure_preparation(&state)?;
        Ok(PlatformWritePermit {
            instance_id: state.instance_id,
            generation: state.generation,
            protocol_version: if state.policy == ProtocolCompatibilityPolicy::Legacy {
                self.protocol_version_number()
            } else {
                state.effective_version
            },
        })
    }

    pub fn validate_platform_write_permit(
        &self,
        permit: &PlatformWritePermit,
    ) -> Result<(), Error> {
        let state = self.compatibility_lock();
        self.ensure_preparation(&state)?;
        if state.policy == ProtocolCompatibilityPolicy::RequireVerified
            && (state.instance_id != permit.instance_id
                || state.generation != permit.generation
                || state.effective_version != permit.protocol_version)
        {
            return Err(ProtocolCompatibilityError::PreparationExpired.into());
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::platform::Query;
    use crate::SdkBuilder;
    use dapi_grpc::platform::v0::{GetEpochsInfoRequest, ResponseMetadata};
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;

    fn sdk() -> Sdk {
        let sdk = SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(13).expect("seed version"))
            .with_height_tolerance(None)
            .build()
            .expect("mock SDK");
        sdk.set_protocol_compatibility_policy(ProtocolCompatibilityPolicy::RequireVerified);
        sdk
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

    #[tokio::test]
    async fn should_require_one_explicit_proven_query_and_share_readiness_with_clones() {
        let mut sdk = sdk();
        let wallet_clone = sdk.clone();
        assert!(matches!(
            wallet_clone.ensure_platform_write_compatible(),
            Err(Error::ProtocolCompatibility(
                ProtocolCompatibilityError::Unavailable
            ))
        ));
        expect_check(&mut sdk).await;
        let generation = sdk.begin_protocol_compatibility_check();
        assert!(matches!(
            wallet_clone.ensure_platform_write_compatible(),
            Err(Error::ProtocolCompatibility(
                ProtocolCompatibilityError::Checking
            ))
        ));
        let result = sdk.refresh_protocol_compatibility(generation).await;
        assert_eq!(result.state, ProtocolCompatibilityState::Supported);
        assert_eq!(
            result.verified_protocol_version,
            Some(PlatformVersion::latest().protocol_version)
        );
        assert_eq!(
            wallet_clone
                .platform_version_for_write()
                .unwrap()
                .protocol_version,
            PlatformVersion::latest().protocol_version
        );
        // No second expectation exists: asking readiness/permits performs no I/O.
        wallet_clone.platform_write_permit().expect("ready clone");
        let request: GetEpochsInfoRequest = 0u16.query(&sdk.query_settings()).unwrap();
        assert_eq!(sdk.mock().request_count(&request).await, 1);
    }

    #[test]
    fn should_never_trust_public_plain_metadata_for_strict_readiness_or_construction() {
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
        assert!(matches!(
            sdk.protocol_compatibility().state,
            ProtocolCompatibilityState::Unavailable { .. }
        ));
        // Even a known version accepted by the legacy helper cannot influence
        // strict construction. Only the private verified path owns that value.
        sdk.verify_response_metadata(
            "get_epochs_info",
            &ResponseMetadata {
                protocol_version: PlatformVersion::latest().protocol_version,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(sdk.protocol_compatibility().effective_protocol_version, 13);
    }

    #[test]
    fn should_keep_unknown_authenticated_versions_sticky_without_constructing_with_them() {
        let sdk = sdk();
        let unknown = PlatformVersion::latest().protocol_version + 1;
        verified(&sdk, unknown);
        verified(&sdk, 13);
        sdk.begin_protocol_compatibility_check();
        assert_eq!(
            sdk.protocol_compatibility().state,
            ProtocolCompatibilityState::Unsupported
        );
        assert_eq!(
            sdk.protocol_compatibility().verified_protocol_version,
            Some(unknown)
        );
        assert_eq!(sdk.protocol_compatibility().effective_protocol_version, 13);
        assert!(
            matches!(sdk.ensure_platform_write_compatible(), Err(Error::ProtocolCompatibility(ProtocolCompatibilityError::Unsupported { network_version, .. })) if network_version == unknown)
        );
    }

    #[tokio::test]
    async fn should_leave_an_unavailable_check_closed_after_ordinary_verified_reads() {
        let sdk = sdk();
        verified(&sdk, 13);
        assert!(matches!(
            sdk.protocol_compatibility().state,
            ProtocolCompatibilityState::Unavailable { .. }
        ));
        let generation = sdk.begin_protocol_compatibility_check();
        // No network expectation -> failed explicit query.
        let result = sdk.refresh_protocol_compatibility(generation).await;
        assert!(matches!(
            result.state,
            ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::CheckFailed
            }
        ));
        verified(&sdk, PlatformVersion::latest().protocol_version);
        assert!(sdk.ensure_platform_write_compatible().is_err());
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
            ProtocolCompatibilityState::Supported
        );
    }

    #[tokio::test]
    async fn should_expire_the_total_budget_including_time_before_worker_execution() {
        let sdk = sdk();
        let generation = sdk.begin_protocol_compatibility_check();
        sdk.protocol_compatibility.lock().unwrap().deadline =
            Some(Instant::now() - Duration::from_secs(1));
        let result = sdk.refresh_protocol_compatibility(generation).await;
        assert!(matches!(
            result.state,
            ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::TimedOut
            }
        ));
        verified(&sdk, 13);
        assert!(sdk.platform_write_permit().is_err());
    }

    #[tokio::test]
    async fn should_timeout_while_a_query_is_pending_and_cancel_that_query() {
        let sdk = sdk();
        let super::super::SdkInstance::Mock { dapi, .. } = &sdk.inner else {
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
    async fn should_ignore_superseded_success_error_and_unknown_version_results() {
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
        assert_eq!(sdk.protocol_compatibility().verified_protocol_version, None);
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
            ProtocolCompatibilityState::Supported
        );
    }

    #[tokio::test]
    async fn should_invalidate_preparation_on_refresh_and_same_network_runtime_replacement() {
        let mut sdk = sdk();
        expect_check(&mut sdk).await;
        let generation = sdk.begin_protocol_compatibility_check();
        sdk.refresh_protocol_compatibility(generation).await;
        let permit = sdk.platform_write_permit().unwrap();
        let prepared_sdk = sdk.platform_write_context().unwrap();
        // Mock expectations are reusable; this issues the same one-query check.
        let next = sdk.begin_protocol_compatibility_check();
        sdk.refresh_protocol_compatibility(next).await;
        assert!(matches!(
            sdk.validate_platform_write_permit(&permit),
            Err(Error::ProtocolCompatibility(
                ProtocolCompatibilityError::PreparationExpired
            ))
        ));
        assert!(matches!(
            prepared_sdk.platform_write_permit(),
            Err(Error::ProtocolCompatibility(
                ProtocolCompatibilityError::PreparationExpired
            ))
        ));
        let mut replacement = super::tests::sdk();
        expect_check(&mut replacement).await;
        let generation = replacement.begin_protocol_compatibility_check();
        replacement.refresh_protocol_compatibility(generation).await;
        assert!(replacement.validate_platform_write_permit(&permit).is_err());
    }

    #[tokio::test]
    async fn should_refuse_queued_confirmation_after_a_newer_check_completes() {
        use crate::platform::transition::broadcast::BroadcastStateTransition;
        use dpp::state_transition::batch_transition::BatchTransitionV0;
        use dpp::state_transition::StateTransition;

        let mut sdk = sdk();
        expect_check(&mut sdk).await;
        let first = sdk.begin_protocol_compatibility_check();
        sdk.refresh_protocol_compatibility(first).await;
        let confirmed = sdk.platform_write_permit().unwrap();
        // Construct the work now but do not poll it until the queue advances.
        let queued = with_platform_write_permit(confirmed, async {
            assert!(matches!(
                sdk.platform_write_context(),
                Err(Error::ProtocolCompatibility(
                    ProtocolCompatibilityError::PreparationExpired
                ))
            ));
            let transition = StateTransition::Batch(BatchTransitionV0::default().into());
            assert!(matches!(
                transition.broadcast(&sdk, None).await,
                Err(Error::ProtocolCompatibility(
                    ProtocolCompatibilityError::PreparationExpired
                ))
            ));
        });
        let second = sdk.begin_protocol_compatibility_check();
        sdk.refresh_protocol_compatibility(second).await;
        assert_eq!(
            sdk.protocol_compatibility().state,
            ProtocolCompatibilityState::Supported
        );
        queued.await;
        // No broadcast expectation exists. The scope also leaves subsequent
        // freshly confirmed work available after the obsolete request ends.
        sdk.platform_write_context().expect("current preparation");
    }

    #[tokio::test]
    async fn should_reject_a_scoped_permit_from_another_runtime() {
        let mut first = sdk();
        expect_check(&mut first).await;
        let generation = first.begin_protocol_compatibility_check();
        first.refresh_protocol_compatibility(generation).await;
        let mut replacement = sdk();
        expect_check(&mut replacement).await;
        let generation = replacement.begin_protocol_compatibility_check();
        replacement.refresh_protocol_compatibility(generation).await;
        with_platform_write_permit(first.platform_write_permit().unwrap(), async {
            assert!(matches!(
                replacement.platform_version_for_write(),
                Err(Error::ProtocolCompatibility(
                    ProtocolCompatibilityError::PreparationExpired
                ))
            ));
        })
        .await;
    }

    #[tokio::test]
    async fn should_preserve_enclosing_confirmation_and_isolate_concurrent_scopes() {
        let mut sdk = sdk();
        expect_check(&mut sdk).await;
        let generation = sdk.begin_protocol_compatibility_check();
        sdk.refresh_protocol_compatibility(generation).await;
        let old = sdk.platform_write_permit().unwrap();
        let generation = sdk.begin_protocol_compatibility_check();
        sdk.refresh_protocol_compatibility(generation).await;
        let current = sdk.platform_write_permit().unwrap();
        let outdated = with_platform_write_permit(old.clone(), async {
            tokio::task::yield_now().await;
            with_platform_write_permit(current.clone(), async {
                assert!(matches!(
                    sdk.platform_write_context(),
                    Err(Error::ProtocolCompatibility(
                        ProtocolCompatibilityError::PreparationExpired
                    ))
                ));
            })
            .await;
        });
        let fresh = with_platform_write_permit(current.clone(), async {
            tokio::task::yield_now().await;
            sdk.platform_write_context()
                .expect("independent current request");
            with_platform_write_permit(old, async {
                assert!(sdk.platform_write_context().is_err());
            })
            .await;
            sdk.platform_write_context().expect("outer scope restored");
        });
        tokio::join!(outdated, fresh);
        sdk.platform_write_context().expect("all scopes removed");
    }

    #[tokio::test]
    async fn should_leave_legacy_clients_unchanged_inside_a_permit_scope() {
        let sdk = SdkBuilder::new_mock().build().unwrap();
        let unrelated = PlatformWritePermit {
            instance_id: u64::MAX,
            generation: u64::MAX,
            protocol_version: u32::MAX,
        };
        with_platform_write_permit(unrelated, async {
            sdk.platform_write_context()
                .expect("legacy policy does not require a permit");
        })
        .await;
    }

    #[test]
    fn should_invalidate_preparation_when_a_verified_read_raises_the_version() {
        let sdk = sdk();
        verified(&sdk, 13);
        sdk.protocol_compatibility.lock().unwrap().state = ProtocolCompatibilityState::Supported;
        let permit = sdk.platform_write_permit().unwrap();
        verified(&sdk, PlatformVersion::latest().protocol_version);
        assert!(matches!(
            sdk.validate_platform_write_permit(&permit),
            Err(Error::ProtocolCompatibility(
                ProtocolCompatibilityError::PreparationExpired
            ))
        ));
    }

    #[test]
    fn should_keep_legacy_clients_writable_without_an_explicit_check() {
        let sdk = SdkBuilder::new_mock().build().unwrap();
        let permit = sdk.platform_write_permit().unwrap();
        sdk.begin_protocol_compatibility_check();
        sdk.validate_platform_write_permit(&permit).unwrap();
        assert_eq!(
            sdk.platform_version_for_write().unwrap().protocol_version,
            sdk.version().protocol_version
        );
    }

    #[tokio::test]
    async fn should_fail_closed_without_proofs() {
        let sdk = SdkBuilder::new_mock().with_proofs(false).build().unwrap();
        sdk.set_protocol_compatibility_policy(ProtocolCompatibilityPolicy::RequireVerified);
        let generation = sdk.begin_protocol_compatibility_check();
        assert!(matches!(
            sdk.refresh_protocol_compatibility(generation).await.state,
            ProtocolCompatibilityState::Unavailable {
                reason: ProtocolCompatibilityUnavailableReason::ProofsDisabled
            }
        ));
    }

    #[test]
    fn should_not_learn_stale_authenticated_metadata() {
        let sdk = SdkBuilder::new_mock()
            .with_time_tolerance(Some(1000))
            .build()
            .unwrap();
        sdk.set_protocol_compatibility_policy(ProtocolCompatibilityPolicy::RequireVerified);
        assert!(sdk
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
        assert_eq!(sdk.protocol_compatibility().verified_protocol_version, None);
    }
}
