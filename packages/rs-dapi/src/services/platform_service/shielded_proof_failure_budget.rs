//! Per-source budget for failed Orchard proof verifications.
//!
//! A spend from the shielded pool proves who is spending only through its
//! Halo 2 proof, so Drive's CheckTx cannot charge a sender whose proof fails,
//! and every failed proof still occupies one of the node's few proof slots.
//! DAPI knows where a broadcast comes from, so it meters proof failures per
//! source: a broadcast carrying Orchard actions reserves its action count
//! before it reaches Drive, gets it back unless Drive answers that the proof
//! is invalid, and a source whose budget is spent is refused before Drive
//! does any work. Honest wallets do not produce invalid proofs, so they never
//! spend any budget.
//!
//! Node-local admission policy, not consensus.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use dpp::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dpp::state_transition::unshield_transition::UnshieldTransition;
use dpp::version::PlatformVersion;
use tonic::Request;

/// Consensus code of `InvalidShieldedProofError`, the CheckTx answer for a
/// bundle whose proof or signatures do not verify.
pub(super) const INVALID_SHIELDED_PROOF_CODE: i64 = 40902;

/// Failed actions a source may have outstanding: two of the usual two-action
/// bundles. A wallet whose proofs verify never fails one, so this only has to
/// absorb a broken client retrying by hand.
const FAILED_ACTIONS_ALLOWED: u32 = 4;

/// Time for one failed action to drain, so a source that keeps failing gets
/// two actions verified a minute.
const ACTION_DRAIN_INTERVAL: Duration = Duration::from_secs(30);

/// Sources tracked at once. When full, a source with nothing in flight and the
/// least debt makes room.
const MAX_TRACKED_SOURCES: usize = 16_384;

/// Where a broadcast comes from. An IPv6 source is its /48, the usual size of
/// one site's allocation, so a holder cannot rotate through its own /64s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SourceKey {
    V4(Ipv4Addr),
    V6Prefix48([u8; 6]),
}

impl From<IpAddr> for SourceKey {
    fn from(ip: IpAddr) -> Self {
        match ip.to_canonical() {
            IpAddr::V4(v4) => SourceKey::V4(v4),
            IpAddr::V6(v6) => {
                let octets = v6.octets();
                let mut prefix = [0; 6];
                prefix.copy_from_slice(&octets[..6]);
                SourceKey::V6Prefix48(prefix)
            }
        }
    }
}

impl SourceKey {
    /// The client address the gateway saw. Envoy (`use_remote_address: true`)
    /// appends the downstream address as the last `x-forwarded-for` entry;
    /// earlier entries come from the client and are ignored. The header is
    /// trusted only on a request from a loopback or private peer, which is
    /// where the gateway sits; any other peer is the client itself.
    pub(super) fn of_request<T>(request: &Request<T>) -> Option<Self> {
        let peer = request.remote_addr().map(|address| address.ip());
        let behind_gateway = peer.is_none_or(is_internal);
        let forwarded = behind_gateway
            .then(|| {
                request
                    .metadata()
                    .get_all("x-forwarded-for")
                    .iter()
                    .next_back()
                    .and_then(|value| value.to_str().ok())
                    .and_then(last_forwarded_address)
            })
            .flatten();
        forwarded.or(peer).map(SourceKey::from)
    }
}

fn is_internal(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    }
}

fn last_forwarded_address(header: &str) -> Option<IpAddr> {
    let entry = header.rsplit(',').next()?.trim();
    IpAddr::from_str(entry)
        .ok()
        .or_else(|| SocketAddr::from_str(entry).ok().map(|address| address.ip()))
}

/// Orchard actions whose proof Drive verifies before admitting the transition,
/// or 0 for bytes without any (including bytes that do not decode, which Drive
/// refuses before any proof work).
pub(super) fn orchard_action_count(state_transition_bytes: &[u8]) -> usize {
    let Ok(state_transition) =
        StateTransition::deserialize_from_bytes_untrusted(state_transition_bytes)
    else {
        return 0;
    };
    match state_transition {
        StateTransition::Shield(ShieldTransition::V0(v0)) => v0.actions.len(),
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(v0)) => v0.actions.len(),
        StateTransition::Unshield(UnshieldTransition::V0(v0)) => v0.actions.len(),
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(v0)) => {
            v0.actions.len()
        }
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(v0)) => {
            v0.actions.len()
        }
        StateTransition::IdentityCreateFromShieldedPool(
            IdentityCreateFromShieldedPoolTransition::V0(v0),
        ) => v0.actions.len(),
        StateTransition::ShieldFromIdentity(ShieldFromIdentityTransition::V0(v0)) => {
            v0.actions.len()
        }
        StateTransition::IdentityTopUpFromShieldedPool(
            IdentityTopUpFromShieldedPoolTransition::V0(v0),
        ) => v0.actions.len(),
        _ => 0,
    }
}

#[derive(Debug)]
struct SourceBudget {
    /// When this source's failed actions have fully drained.
    debt_until: Instant,
    /// Actions reserved by broadcasts still waiting for Drive's answer.
    in_flight: u32,
}

impl SourceBudget {
    fn debt(&self, now: Instant) -> Duration {
        self.debt_until.saturating_duration_since(now)
    }
}

/// Per source, failed Orchard actions still draining and the actions of
/// broadcasts still waiting for Drive. A source with no failures may have one
/// largest bundle's worth in flight, which is all the concurrency an honest
/// sender needs. A source that failed recently gets one broadcast at a time,
/// and only while its failures plus that broadcast stay within the allowance,
/// so it cannot burst past the drain rate.
pub struct ShieldedProofFailureBudget {
    sources: Mutex<HashMap<SourceKey, SourceBudget>>,
    failed_actions_allowed: u32,
    max_in_flight_actions: u32,
    drain_interval: Duration,
    max_sources: usize,
}

impl Default for ShieldedProofFailureBudget {
    fn default() -> Self {
        // `latest()` is a static upper bound, as for the size pre-filter:
        // Drive refuses a bundle over the active limit before any proof work.
        let max_actions = PlatformVersion::latest()
            .system_limits
            .max_shielded_transition_actions;
        Self {
            sources: Mutex::new(HashMap::new()),
            failed_actions_allowed: FAILED_ACTIONS_ALLOWED,
            max_in_flight_actions: u32::from(max_actions),
            drain_interval: ACTION_DRAIN_INTERVAL,
            max_sources: MAX_TRACKED_SOURCES,
        }
    }
}

impl ShieldedProofFailureBudget {
    /// Reserve `actions` from `source`'s budget for one broadcast. `Ok(None)`
    /// for a broadcast without Orchard actions; `Err` carries how long the
    /// source should wait before trying again.
    pub(super) fn try_reserve(
        &self,
        source: SourceKey,
        actions: usize,
    ) -> Result<Option<ProofFailureReservation<'_>>, Duration> {
        self.try_reserve_at(source, actions, Instant::now())
    }

    fn try_reserve_at(
        &self,
        source: SourceKey,
        actions: usize,
        now: Instant,
    ) -> Result<Option<ProofFailureReservation<'_>>, Duration> {
        if actions == 0 {
            return Ok(None);
        }
        // Drive refuses more actions than the system limit before any proof
        // work, so a larger bundle costs no more than the largest valid one.
        let actions = u32::try_from(actions)
            .unwrap_or(u32::MAX)
            .min(self.max_in_flight_actions);

        let mut sources = self
            .sources
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(entry) = sources.get(&source) {
            let debt = entry.debt(now);
            if debt.is_zero() {
                if entry.in_flight + actions > self.max_in_flight_actions {
                    return Err(Duration::from_secs(1));
                }
            } else {
                if entry.in_flight != 0 {
                    return Err(Duration::from_secs(1));
                }
                // A bundle larger than the allowance waits for a clean slate.
                let room = (self.drain_interval * self.failed_actions_allowed)
                    .saturating_sub(self.drain_interval * actions);
                if debt > room {
                    return Err((debt - room).max(Duration::from_secs(1)));
                }
            }
        } else if sources.len() >= self.max_sources {
            sources.retain(|_, entry| entry.in_flight != 0 || entry.debt_until > now);
            if sources.len() >= self.max_sources {
                let victim = sources
                    .iter()
                    .filter(|(_, entry)| entry.in_flight == 0)
                    .min_by_key(|(_, entry)| entry.debt_until)
                    .map(|(key, _)| *key);
                // Every tracked source has a broadcast in flight: the node is
                // saturated, so refuse rather than lose track of one.
                let Some(victim) = victim else {
                    return Err(Duration::from_secs(1));
                };
                sources.remove(&victim);
            }
        }

        let entry = sources.entry(source).or_insert(SourceBudget {
            debt_until: now,
            in_flight: 0,
        });
        entry.in_flight += actions;
        Ok(Some(ProofFailureReservation {
            budget: self,
            source,
            actions,
            settled: false,
        }))
    }

    fn settle(&self, source: SourceKey, actions: u32, proof_failed: bool, now: Instant) {
        let mut sources = self
            .sources
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // A source is never dropped while it has a broadcast in flight.
        let Some(entry) = sources.get_mut(&source) else {
            return;
        };
        entry.in_flight = entry.in_flight.saturating_sub(actions);
        if proof_failed {
            entry.debt_until = entry.debt_until.max(now) + self.drain_interval * actions;
        }
    }
}

/// Actions reserved for one broadcast. Dropping it unsettled charges the
/// source as for a failed proof: a client that disconnects mid-broadcast may
/// still have cost Drive a verification.
#[must_use]
pub(super) struct ProofFailureReservation<'a> {
    budget: &'a ShieldedProofFailureBudget,
    source: SourceKey,
    actions: u32,
    settled: bool,
}

impl ProofFailureReservation<'_> {
    /// Drive answered that the proof is invalid: the actions stay as debt.
    pub(super) fn charge(self) {
        self.settle_at(true, Instant::now());
    }

    /// Drive answered anything else: the actions return to the budget.
    pub(super) fn refund(self) {
        self.settle_at(false, Instant::now());
    }

    fn settle_at(mut self, proof_failed: bool, now: Instant) {
        self.settled = true;
        self.budget
            .settle(self.source, self.actions, proof_failed, now);
    }
}

impl Drop for ProofFailureReservation<'_> {
    fn drop(&mut self) {
        if !self.settled {
            self.budget
                .settle(self.source, self.actions, true, Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::ConsensusError;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use std::net::Ipv6Addr;

    const SOURCE: SourceKey = SourceKey::V4(Ipv4Addr::new(203, 0, 113, 7));

    fn budget() -> ShieldedProofFailureBudget {
        ShieldedProofFailureBudget::default()
    }

    #[test]
    fn should_pin_the_invalid_shielded_proof_code() {
        let error = ConsensusError::from(StateError::InvalidShieldedProofError(
            InvalidShieldedProofError::new(String::new()),
        ));
        assert_eq!(i64::from(error.code()), INVALID_SHIELDED_PROOF_CODE);
    }

    #[test]
    fn should_not_meter_broadcasts_without_orchard_actions() {
        let budget = budget();
        let now = Instant::now();
        for _ in 0..1_000 {
            assert!(budget.try_reserve_at(SOURCE, 0, now).unwrap().is_none());
        }
    }

    /// Reserve for one broadcast and settle it, as the broadcast handler does.
    fn broadcast(
        budget: &ShieldedProofFailureBudget,
        source: SourceKey,
        actions: usize,
        proof_failed: bool,
        now: Instant,
    ) -> Result<(), Duration> {
        budget
            .try_reserve_at(source, actions, now)?
            .expect("orchard actions are metered")
            .settle_at(proof_failed, now);
        Ok(())
    }

    #[test]
    fn should_never_refuse_a_source_whose_proofs_verify() {
        let budget = budget();
        let now = Instant::now();
        for _ in 0..1_000 {
            broadcast(&budget, SOURCE, 16, false, now)
                .expect("verified proofs never spend the allowance");
        }
    }

    #[test]
    fn should_refuse_a_source_after_two_failed_bundles() {
        let budget = budget();
        let now = Instant::now();
        broadcast(&budget, SOURCE, 2, true, now).unwrap();
        broadcast(&budget, SOURCE, 2, true, now).unwrap();

        assert_eq!(
            broadcast(&budget, SOURCE, 2, true, now),
            Err(ACTION_DRAIN_INTERVAL * 2),
            "the third bundle waits until one bundle has drained"
        );
        let other = SourceKey::V4(Ipv4Addr::new(198, 51, 100, 1));
        assert!(
            broadcast(&budget, other, 2, false, now).is_ok(),
            "another source keeps its own allowance"
        );
    }

    #[test]
    fn should_verify_two_actions_a_minute_for_a_source_that_keeps_failing() {
        let budget = budget();
        let now = Instant::now();
        broadcast(&budget, SOURCE, 2, true, now).unwrap();
        broadcast(&budget, SOURCE, 2, true, now).unwrap();

        for minute in 1..=10 {
            let due = now + Duration::from_secs(60 * minute);
            assert!(broadcast(&budget, SOURCE, 2, true, due - Duration::from_secs(1)).is_err());
            broadcast(&budget, SOURCE, 2, true, due).expect("one bundle a minute");
        }
    }

    #[test]
    fn should_admit_one_broadcast_at_a_time_after_a_failure() {
        let budget = budget();
        let now = Instant::now();
        broadcast(&budget, SOURCE, 1, true, now).unwrap();

        let in_flight = budget.try_reserve_at(SOURCE, 1, now).unwrap().unwrap();
        assert!(
            budget.try_reserve_at(SOURCE, 1, now).is_err(),
            "a failing source cannot fan out concurrent broadcasts"
        );
        in_flight.settle_at(false, now);
        assert!(broadcast(&budget, SOURCE, 1, false, now).is_ok());
    }

    #[test]
    fn should_cap_actions_in_flight_at_one_largest_bundle() {
        let budget = budget();
        let now = Instant::now();
        let largest = budget.try_reserve_at(SOURCE, 16, now).unwrap().unwrap();
        assert!(
            budget.try_reserve_at(SOURCE, 1, now).is_err(),
            "concurrent broadcasts must not all pass before any failure is charged"
        );
        largest.settle_at(false, now);
        assert!(broadcast(&budget, SOURCE, 16, false, now).is_ok());
    }

    #[test]
    fn should_make_a_failed_largest_bundle_wait_for_a_clean_slate() {
        let budget = budget();
        let now = Instant::now();
        broadcast(&budget, SOURCE, 16, true, now).unwrap();

        let drained = now + ACTION_DRAIN_INTERVAL * 16;
        assert!(broadcast(&budget, SOURCE, 16, false, drained - Duration::from_secs(1)).is_err());
        assert!(broadcast(&budget, SOURCE, 16, false, drained).is_ok());
    }

    #[test]
    fn should_charge_a_reservation_dropped_unsettled() {
        let budget = budget();
        drop(budget.try_reserve(SOURCE, 16).unwrap().unwrap());
        assert!(
            budget.try_reserve(SOURCE, 2).is_err(),
            "a cancelled broadcast may still have cost Drive a verification"
        );
    }

    #[test]
    fn should_charge_an_oversized_bundle_as_the_largest_valid_one() {
        let budget = budget();
        let now = Instant::now();
        broadcast(&budget, SOURCE, 1_000, true, now).expect("a clean source is admitted");
        assert_eq!(
            broadcast(&budget, SOURCE, 2, true, now),
            Err(ACTION_DRAIN_INTERVAL * 14),
            "sixteen failed actions, less room for one two-action bundle"
        );
    }

    #[test]
    fn should_evict_the_least_indebted_idle_source_when_full() {
        let mut budget = budget();
        budget.max_sources = 2;
        let now = Instant::now();
        let a = SourceKey::V4(Ipv4Addr::new(192, 0, 2, 1));
        let b = SourceKey::V4(Ipv4Addr::new(192, 0, 2, 2));
        let c = SourceKey::V4(Ipv4Addr::new(192, 0, 2, 3));
        broadcast(&budget, a, 16, true, now).unwrap();
        broadcast(&budget, b, 1, true, now).unwrap();

        budget
            .try_reserve_at(c, 1, now)
            .expect("a new source displaces the least indebted one")
            .unwrap()
            .settle_at(false, now);
        assert!(
            budget.try_reserve_at(a, 1, now).is_err(),
            "the most indebted source stays tracked"
        );
    }

    #[test]
    fn should_refuse_a_new_source_when_every_tracked_source_is_in_flight() {
        let mut budget = budget();
        budget.max_sources = 1;
        let now = Instant::now();
        let in_flight = budget.try_reserve_at(SOURCE, 1, now).unwrap().unwrap();
        let other = SourceKey::V4(Ipv4Addr::new(192, 0, 2, 9));
        assert!(budget.try_reserve_at(other, 1, now).is_err());
        in_flight.settle_at(false, now);
        assert!(budget.try_reserve_at(other, 1, now).is_ok());
    }

    #[test]
    fn should_group_ipv6_sources_by_48_bit_prefix() {
        let one: IpAddr = "2001:db8:1:aaaa::1".parse().unwrap();
        let same_site: IpAddr = "2001:db8:1:bbbb::2".parse().unwrap();
        let other_site: IpAddr = "2001:db8:2::1".parse().unwrap();
        assert_eq!(SourceKey::from(one), SourceKey::from(same_site));
        assert_ne!(SourceKey::from(one), SourceKey::from(other_site));

        let mapped = IpAddr::V6(Ipv4Addr::new(203, 0, 113, 7).to_ipv6_mapped());
        assert_eq!(SourceKey::from(mapped), SOURCE);
    }

    fn request_from(peer: &str, forwarded_for: Option<&str>) -> Request<()> {
        let mut request = Request::new(());
        request
            .extensions_mut()
            .insert(tonic::transport::server::TcpConnectInfo {
                local_addr: None,
                remote_addr: Some(peer.parse::<SocketAddr>().unwrap()),
            });
        if let Some(value) = forwarded_for {
            request
                .metadata_mut()
                .insert("x-forwarded-for", value.parse().unwrap());
        }
        request
    }

    #[test]
    fn should_take_the_gateway_appended_address_behind_the_gateway() {
        let request = request_from("172.18.0.5:41000", Some("10.9.9.9, 203.0.113.7"));
        assert_eq!(SourceKey::of_request(&request), Some(SOURCE));
    }

    #[test]
    fn should_ignore_forwarded_for_from_a_public_peer() {
        let request = request_from("198.51.100.1:41000", Some("203.0.113.7"));
        assert_eq!(
            SourceKey::of_request(&request),
            Some(SourceKey::V4(Ipv4Addr::new(198, 51, 100, 1)))
        );
    }

    #[test]
    fn should_fall_back_to_the_peer_without_forwarded_for() {
        let request = request_from("[fd00::5]:41000", None);
        assert_eq!(
            SourceKey::of_request(&request),
            Some(SourceKey::from(IpAddr::V6(Ipv6Addr::new(
                0xfd00, 0, 0, 0, 0, 0, 0, 5
            ))))
        );
    }

    #[test]
    fn should_count_the_actions_of_a_shielded_transfer() {
        let transfer = StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: vec![
                    SerializedAction {
                        nullifier: [1; 32],
                        rk: [2; 32],
                        cmx: [3; 32],
                        encrypted_note: vec![4; 216],
                        cv_net: [5; 32],
                        spend_auth_sig: [6; 64],
                    };
                    3
                ],
                value_balance: 1,
                anchor: [7; 32],
                proof: vec![0; 100],
                binding_signature: [0; 64],
            },
        ));
        let bytes = transfer.serialize_to_bytes().unwrap();
        assert_eq!(orchard_action_count(&bytes), 3);
        assert_eq!(orchard_action_count(&[0xff, 0x00]), 0);
    }
}
