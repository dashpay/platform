//! Proofs of the state before and after the block that executed a state transition.
//!
//! A proof of a state transition's keys shows the state at the block the proof was made at.
//! Made when a client asks, it is of whatever block the node has reached by then. To show
//! what the transition's block changed, two proofs of the same keys are needed: one of the
//! state the previous block left, and one of the state the transition's own block left.
//!
//! The first can only be made before the transition's block is committed, so a node has to
//! know in advance which transitions to prove. A client registers a transition by its hash
//! for as long as it waits for the transition, and unregisters it when it stops waiting.
//! When a block that holds a registered transition is finalized, the node proves the
//! transition's keys before it commits the block and again after, and keeps the pair for the
//! client to fetch.
//!
//! None of this is consensus. It reads committed state, writes nothing to it, and a failure
//! to prove never fails a block. The bounds below are this node's own limits: what it keeps in
//! memory, and how much proving it adds to finalizing a block.

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::query::response_metadata::CheckpointUsed;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use dpp::util::hash::hash_single;
use drive::drive::Drive;
use drive::util::grove_operations::GroveDBToUse;
use drive::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// How many registrations can be held at once, one per waiting client. A registration past
/// this is refused.
///
/// It matches the number of waits one rs-dapi holds at once, so every wait of the rs-dapi in
/// front of this node can hold one.
pub const MAX_REGISTERED_STATE_TRANSITIONS: usize = 1024;

/// How many blocks a registration lives when nothing releases it, as when the client that took
/// it stopped without saying so. Each registration expires on its own.
pub const REGISTRATION_LIFETIME_BLOCKS: u64 = 64;

/// How many blocks a proof is kept for after the block it is of.
pub const PROOF_RETENTION_BLOCKS: u64 = 64;

/// How many registered transitions of one block are proved. Each costs two proofs while the
/// block is finalized, so this bounds what registrations add to finalizing a block, together
/// with proving only the kinds whose proof is a small fixed set of keys. The registered
/// transitions of a block past this are not proved, in block order.
pub const MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK: usize = 32;

/// How many bytes of proofs are kept at once. A pair that would go past this is not kept, and
/// its client gets a proof of current state instead.
pub const MAX_RETAINED_PROOF_BYTES: usize = 16 * 1024 * 1024;

/// How many releases that arrived before their own registration are remembered at once, so that
/// the registration is not taken when it arrives. Only such out-of-order releases are
/// remembered: a release of a held registration needs no memory. When the table is full, the
/// release with the lowest sequence number is forgotten and its issuer's low-water mark raised
/// to it, so it and every earlier lease of that issuer are still never taken, while the issuer's
/// later leases are.
pub const MAX_REMEMBERED_RELEASES: usize = MAX_REGISTERED_STATE_TRANSITIONS;

/// How many issuers' low-water marks are kept at once. Past this the oldest is forgotten, and a
/// late registration under it is taken and expires after [`REGISTRATION_LIFETIME_BLOCKS`].
pub const MAX_RELEASE_ISSUERS: usize = 64;

/// The hash a state transition is registered and fetched by: the hash of its bytes, which is
/// also its transaction hash in a block.
pub type StateTransitionHash = [u8; 32];

/// What one waiting client's registration is known by: 8 bytes naming its issuer (one rs-dapi
/// process) and an 8-byte big-endian sequence number the issuer increases for every lease. Each
/// lease is registered once, so taking and releasing it are idempotent, and a release only ever
/// ends its own registration. A caller does not register a lease again, even when it did not
/// learn whether the first attempt arrived: a lease registered again after its release is taken.
pub type RegistrationLease = [u8; 16];

/// The issuer of a lease, and its sequence number.
fn lease_parts(lease: &RegistrationLease) -> ([u8; 8], u64) {
    let mut issuer = [0; 8];
    issuer.copy_from_slice(&lease[..8]);
    let mut sequence = [0; 8];
    sequence.copy_from_slice(&lease[8..]);
    (issuer, u64::from_be_bytes(sequence))
}

/// A proof of a state transition's keys, with the block whose state it is of. The metadata
/// and the signature in the proof are that block's, as in any proved query response.
#[derive(Clone, Debug, PartialEq)]
pub struct ProofOfStateAtBlock {
    /// The GroveDB proof and the quorum signature of the block.
    pub proof: Proof,
    /// The block the proof is of.
    pub metadata: ResponseMetadata,
}

/// The state of a state transition's keys before and after the block that executed it.
#[derive(Clone, Debug, PartialEq)]
pub struct StateTransitionExecutionProof {
    /// The state the block before the transition's block left.
    pub before: ProofOfStateAtBlock,
    /// The state the transition's block left.
    pub after: ProofOfStateAtBlock,
}

impl StateTransitionExecutionProof {
    /// The bytes the pair holds, as counted against [`MAX_RETAINED_PROOF_BYTES`].
    fn retained_bytes(&self) -> usize {
        [&self.before, &self.after]
            .iter()
            .map(|side| {
                side.proof.grovedb_proof.len()
                    + side.proof.quorum_hash.len()
                    + side.proof.signature.len()
                    + side.proof.block_id_hash.len()
                    + side.metadata.chain_id.len()
            })
            .sum()
    }
}

/// The proof of the state before a block, made before the block is committed and waiting for
/// the proof of the state after it.
#[derive(Debug)]
pub struct PendingStateTransitionExecutionProof {
    hash: StateTransitionHash,
    state_transition: StateTransition,
    before: ProofOfStateAtBlock,
}

/// Why a state transition could not be registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationRefused {
    /// [`MAX_REGISTERED_STATE_TRANSITIONS`] registrations are held already.
    TooManyRegistered,
    /// The lease's release arrived first, or the lease is no newer than a release the node has
    /// forgotten (its issuer's low-water mark), so the node holds nothing for it.
    LeaseReleased,
}

#[derive(Debug, Default)]
struct Inner {
    /// The registered transitions: the registrations waiting for each, with the height of the
    /// last committed block when each was taken.
    registered: HashMap<StateTransitionHash, HashMap<RegistrationLease, u64>>,
    /// How many registrations `registered` holds.
    registrations: usize,
    /// Releases that arrived before their registration, with the height of the last committed
    /// block then.
    released: HashMap<RegistrationLease, u64>,
    /// For each issuer whose early releases did not all fit in `released`: the highest sequence
    /// number forgotten, at or below which none of its leases is taken, with the height of the
    /// last committed block when it was raised.
    release_low_water_marks: HashMap<[u8; 8], (u64, u64)>,
    /// The proofs made, with the height of the block each transition was executed in.
    proved: HashMap<StateTransitionHash, (u64, Arc<StateTransitionExecutionProof>)>,
    /// The bytes `proved` holds.
    proved_bytes: usize,
    /// The height of the last block committed.
    last_committed_height: u64,
}

impl Inner {
    /// Remembers a release that arrived before its registration. When the table is full, the
    /// release with the lowest sequence number of all is forgotten instead, and its issuer's
    /// low-water mark raised to it.
    fn remember_early_release(&mut self, lease: RegistrationLease) {
        if self.released.contains_key(&lease) || self.below_low_water_mark(&lease) {
            return;
        }
        let at = self.last_committed_height;
        self.released.insert(lease, at);
        if self.released.len() <= MAX_REMEMBERED_RELEASES {
            return;
        }
        let Some(forgotten) = self
            .released
            .keys()
            .min_by_key(|lease| lease_parts(lease).1)
            .copied()
        else {
            return;
        };
        self.released.remove(&forgotten);
        let (issuer, sequence) = lease_parts(&forgotten);
        if !self.release_low_water_marks.contains_key(&issuer)
            && self.release_low_water_marks.len() >= MAX_RELEASE_ISSUERS
        {
            if let Some(oldest) = self
                .release_low_water_marks
                .iter()
                .min_by_key(|(_, (_, raised_at))| *raised_at)
                .map(|(issuer, _)| *issuer)
            {
                self.release_low_water_marks.remove(&oldest);
            }
        }
        let mark = self
            .release_low_water_marks
            .entry(issuer)
            .or_insert((sequence, at));
        *mark = (mark.0.max(sequence), at);
    }

    /// Whether this lease is at or below its issuer's low-water mark, so it was released or
    /// lies before a release that was.
    fn below_low_water_mark(&self, lease: &RegistrationLease) -> bool {
        let (issuer, sequence) = lease_parts(lease);
        self.release_low_water_marks
            .get(&issuer)
            .is_some_and(|(mark, _)| sequence <= *mark)
    }

    /// Whether the state transition with this hash waits for its block: it is registered and
    /// not proved yet.
    fn waits_for_block(&self, hash: &StateTransitionHash) -> bool {
        self.registered.contains_key(hash) && !self.proved.contains_key(hash)
    }
}

/// The state transitions a node was asked to prove, and the proofs it has made of them.
#[derive(Debug, Default)]
pub struct StateTransitionExecutionProofs {
    inner: Mutex<Inner>,
}

impl StateTransitionExecutionProofs {
    /// The registry of a node whose last committed block is at `last_committed_height`, so that
    /// registrations taken before the next block count their lifetime from there.
    pub fn new(last_committed_height: u64) -> Self {
        Self {
            inner: Mutex::new(Inner {
                last_committed_height,
                ..Default::default()
            }),
        }
    }

    /// A panic while the lock is held leaves the maps as they were between two operations, each
    /// of which leaves them consistent, so a poisoned lock is still safe to use.
    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Asks the node to prove the state before and after the block that executes the state
    /// transition with this hash, for as long as the registration `lease` is held. Several
    /// clients can wait for the same transition, each with its own lease.
    ///
    /// Each lease is sent in one registration only. A caller that does not learn whether its
    /// registration arrived releases the lease and takes a new one for any further attempt: an
    /// ordinary release forgets the lease, so registering it again would take it again. Taking
    /// a lease that is held already changes nothing. A lease whose release arrived first, or
    /// that is no newer than its issuer's low-water mark, is refused with
    /// [`RegistrationRefused::LeaseReleased`], so a caller still waiting learns that the node
    /// holds nothing for it. A lease is held until it is released or expires, also once its
    /// transition is proved, so that its release finds it.
    pub fn register(
        &self,
        hash: StateTransitionHash,
        lease: RegistrationLease,
    ) -> Result<(), RegistrationRefused> {
        let mut inner = self.inner();
        if inner.released.contains_key(&lease) || inner.below_low_water_mark(&lease) {
            return Err(RegistrationRefused::LeaseReleased);
        }
        if inner
            .registered
            .get(&hash)
            .is_some_and(|leases| leases.contains_key(&lease))
        {
            return Ok(());
        }
        if inner.registrations >= MAX_REGISTERED_STATE_TRANSITIONS {
            return Err(RegistrationRefused::TooManyRegistered);
        }
        let taken_at = inner.last_committed_height;
        inner
            .registered
            .entry(hash)
            .or_default()
            .insert(lease, taken_at);
        inner.registrations += 1;
        Ok(())
    }

    /// Releases the registration `lease` of the state transition with this hash, when the
    /// client that took it stops waiting. The transition stays registered while another
    /// registration of it is held. Releasing a lease again changes nothing, and a proof already
    /// made is kept.
    pub fn unregister(&self, hash: &StateTransitionHash, lease: &RegistrationLease) {
        let mut inner = self.inner();
        let removed = match inner.registered.get_mut(hash) {
            Some(leases) => {
                let removed = leases.remove(lease).is_some();
                if leases.is_empty() {
                    inner.registered.remove(hash);
                }
                removed
            }
            None => false,
        };
        if removed {
            inner.registrations -= 1;
        } else {
            // The release came first, or the lease expired already: its registration may still
            // be on its way, and must not be taken when it arrives
            inner.remember_early_release(*lease);
        }
    }

    /// Whether the state transition with this hash is waiting for its block.
    pub fn is_registered(&self, hash: &StateTransitionHash) -> bool {
        self.inner().waits_for_block(hash)
    }

    /// Whether any state transition is waiting for its block.
    pub fn has_registered(&self) -> bool {
        let inner = self.inner();
        inner
            .registered
            .keys()
            .any(|hash| inner.waits_for_block(hash))
    }

    /// The proof of the state transition with this hash, if the node made one and still keeps
    /// it.
    pub fn get(&self, hash: &StateTransitionHash) -> Option<Arc<StateTransitionExecutionProof>> {
        self.inner()
            .proved
            .get(hash)
            .map(|(_, proof)| Arc::clone(proof))
    }

    /// Keeps the proof of a state transition executed in the block at `height`, unless keeping
    /// it would go past [`MAX_RETAINED_PROOF_BYTES`]. The registrations of the transition stay
    /// held until their clients release them.
    pub(crate) fn store(
        &self,
        hash: StateTransitionHash,
        height: u64,
        proof: StateTransitionExecutionProof,
    ) {
        let mut inner = self.inner();
        let bytes = proof.retained_bytes();
        if inner.proved_bytes.saturating_add(bytes) > MAX_RETAINED_PROOF_BYTES {
            tracing::warn!(
                state_transition_hash = hex::encode(hash),
                bytes,
                retained = inner.proved_bytes,
                "the proofs kept already fill their budget; this pair is not kept"
            );
            return;
        }
        inner.proved_bytes += bytes;
        if let Some((_, replaced)) = inner.proved.insert(hash, (height, Arc::new(proof))) {
            inner.proved_bytes -= replaced.retained_bytes();
        }
    }

    /// Records that the block at `height` is committed, and forgets the registrations, the
    /// releases and the proofs that have outlived their bounds.
    fn block_committed(&self, height: u64) {
        let mut inner = self.inner();
        inner.last_committed_height = height;
        let live = |at: &u64| height.saturating_sub(*at) <= REGISTRATION_LIFETIME_BLOCKS;

        let mut expired = 0;
        inner.registered.retain(|_, leases| {
            let held = leases.len();
            leases.retain(|_, taken_at| live(taken_at));
            expired += held - leases.len();
            !leases.is_empty()
        });
        inner.registrations -= expired;
        inner.released.retain(|_, released_at| live(released_at));
        // A mark is kept as long as a remembered release would be
        inner
            .release_low_water_marks
            .retain(|_, (_, raised_at)| live(raised_at));

        let mut freed = 0;
        inner.proved.retain(|_, (proved_at, proof)| {
            let kept = height.saturating_sub(*proved_at) <= PROOF_RETENTION_BLOCKS;
            if !kept {
                freed += proof.retained_bytes();
            }
            kept
        });
        inner.proved_bytes -= freed;
    }
}

impl<C> Platform<C> {
    /// Proves the keys of the registered state transitions among `raw_state_transitions`, the
    /// transactions of a block that is about to be committed.
    ///
    /// It must be called before anything of the block is committed and before the platform
    /// state takes the block in: the committed state and the platform state are then both the
    /// previous block's, so each proof is of the state the previous block left and carries that
    /// block's signature.
    ///
    /// Only the kinds whose state before them a client can read are proved
    /// (`Drive::state_transition_prior_state_is_readable`), and at most
    /// [`MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK`] of them, in block order. A transition skipped
    /// for either reason, or one that cannot be proved, stays registered until its client
    /// releases it.
    pub(crate) fn prove_registered_state_transitions_before_block(
        &self,
        raw_state_transitions: &[Vec<u8>],
    ) -> Vec<PendingStateTransitionExecutionProof> {
        if !self.state_transition_execution_proofs.has_registered() {
            return Vec::new();
        }

        let platform_state = self.state.load();
        // Without a committed block there is no signed state to prove against
        if platform_state.last_committed_block_info().is_none() {
            return Vec::new();
        }

        let platform_version = match platform_state.current_platform_version() {
            Ok(platform_version) => platform_version,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    "no platform version to prove the registered state transitions with"
                );
                return Vec::new();
            }
        };

        // Only the kinds whose state before them a client can read are proved: their proofs
        // are a small fixed set of keys, so each costs a bounded amount of work. The scan stops
        // once the block's limit is reached, looking one past it only to say so.
        let mut provable = raw_state_transitions
            .iter()
            .map(|raw_state_transition| (hash_single(raw_state_transition), raw_state_transition))
            .filter(|(hash, _)| self.state_transition_execution_proofs.is_registered(hash))
            .filter_map(|(hash, raw_state_transition)| {
                match StateTransition::deserialize_from_bytes_untrusted(raw_state_transition) {
                    Ok(state_transition) => Some((hash, state_transition)),
                    Err(error) => {
                        tracing::warn!(
                            state_transition_hash = hex::encode(hash),
                            ?error,
                            "cannot decode a registered state transition to prove it"
                        );
                        None
                    }
                }
            })
            .filter(|(hash, state_transition)| {
                let readable = Drive::state_transition_prior_state_is_readable(
                    state_transition,
                    platform_version,
                )
                .unwrap_or(false);
                if !readable {
                    tracing::debug!(
                        state_transition_hash = hex::encode(hash),
                        "a registered state transition of a kind whose state before it is not read is not proved"
                    );
                }
                readable
            })
            .take(MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK + 1)
            .collect::<Vec<_>>();

        if provable.len() > MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK {
            tracing::warn!(
                proved = MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK,
                "more registered state transitions in the block than are proved in one block; \
                 the ones past the limit are not proved"
            );
            provable.truncate(MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK);
        }

        provable
            .into_iter()
            .filter_map(|(hash, state_transition)| {
                let before = self.prove_state_at_last_committed_block(
                    &state_transition,
                    &hash,
                    &platform_state,
                )?;
                Some(PendingStateTransitionExecutionProof {
                    hash,
                    state_transition,
                    before,
                })
            })
            .collect()
    }

    /// Proves the keys of the state transitions in `pending` again, now that the block at
    /// `height`, which executed them, is committed and the platform state has taken it in, and
    /// keeps each pair of proofs for its client.
    ///
    /// It must be called before the next block is committed, so that each proof is of the
    /// state the block at `height` left.
    pub(crate) fn prove_registered_state_transitions_after_block(
        &self,
        pending: Vec<PendingStateTransitionExecutionProof>,
        height: u64,
    ) {
        let platform_state = self.state.load();
        if platform_state.last_committed_block_height() == height {
            for PendingStateTransitionExecutionProof {
                hash,
                state_transition,
                before,
            } in pending
            {
                if let Some(after) = self.prove_state_at_last_committed_block(
                    &state_transition,
                    &hash,
                    &platform_state,
                ) {
                    self.state_transition_execution_proofs.store(
                        hash,
                        height,
                        StateTransitionExecutionProof { before, after },
                    );
                }
            }
        } else if !pending.is_empty() {
            tracing::warn!(
                height,
                last_committed_block_height = platform_state.last_committed_block_height(),
                "the platform state is not at the block just committed; \
                 the state transitions registered for a proof are not proved"
            );
        }

        self.state_transition_execution_proofs
            .block_committed(height);
    }

    /// A proof of the transition's keys in committed state, with the signature and the
    /// metadata of the last committed block in `platform_state`. The caller makes sure the
    /// committed state is that block's.
    fn prove_state_at_last_committed_block(
        &self,
        state_transition: &StateTransition,
        hash: &StateTransitionHash,
        platform_state: &PlatformState,
    ) -> Option<ProofOfStateAtBlock> {
        let platform_version = match platform_state.current_platform_version() {
            Ok(platform_version) => platform_version,
            Err(error) => {
                tracing::warn!(
                    state_transition_hash = hex::encode(hash),
                    ?error,
                    "no platform version to prove a registered state transition with"
                );
                return None;
            }
        };

        let grovedb_proof =
            match self
                .drive
                .prove_state_transition(state_transition, None, platform_version)
            {
                Ok(result) if result.is_valid() => result.into_data().ok()?,
                Ok(result) => {
                    tracing::debug!(
                        state_transition_hash = hex::encode(hash),
                        errors = ?result.errors,
                        "a registered state transition cannot be proved"
                    );
                    return None;
                }
                Err(error) => {
                    tracing::warn!(
                        state_transition_hash = hex::encode(hash),
                        ?error,
                        "failed to prove a registered state transition"
                    );
                    return None;
                }
            };

        // A client refuses a GroveDB proof envelope older than its verifier's floor, and with it
        // the whole response, so a side it cannot read is not made. Across a protocol upgrade
        // that changes the envelope no pair is kept, and the wait answers with a proof of the
        // current state instead.
        if let Err(error) =
            require_supported_grovedb_proof_envelope(&grovedb_proof, "execution proof")
        {
            tracing::debug!(
                state_transition_hash = hex::encode(hash),
                ?error,
                "a registered state transition's proof has an envelope clients refuse"
            );
            return None;
        }

        let (_, proof) = self
            .response_proof_v0(platform_state, grovedb_proof, GroveDBToUse::Current)
            .inspect_err(|error| {
                tracing::warn!(
                    state_transition_hash = hex::encode(hash),
                    ?error,
                    "failed to sign off the proof of a registered state transition"
                )
            })
            .ok()?;

        Some(ProofOfStateAtBlock {
            proof,
            metadata: self.response_metadata_v0(platform_state, CheckpointUsed::Current),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: RegistrationLease = [1; 16];
    const B: RegistrationLease = [2; 16];

    fn proof_at(height: u64) -> StateTransitionExecutionProof {
        proof_of_size(height, 0)
    }

    fn proof_of_size(height: u64, grovedb_proof_bytes: usize) -> StateTransitionExecutionProof {
        let side = |height| ProofOfStateAtBlock {
            proof: Proof {
                grovedb_proof: vec![0; grovedb_proof_bytes],
                ..Default::default()
            },
            metadata: ResponseMetadata {
                height,
                ..Default::default()
            },
        };
        StateTransitionExecutionProof {
            before: side(height - 1),
            after: side(height),
        }
    }

    fn hash_of(index: usize) -> StateTransitionHash {
        let mut hash = [0; 32];
        hash[..8].copy_from_slice(&(index as u64).to_be_bytes());
        hash
    }

    fn lease_of(index: usize) -> RegistrationLease {
        let mut lease = [0; 16];
        lease[..8].copy_from_slice(&(index as u64).to_be_bytes());
        lease
    }

    #[test]
    fn should_keep_a_registered_transition_until_it_is_proved() {
        let proofs = StateTransitionExecutionProofs::default();
        assert!(!proofs.has_registered());

        proofs.register([1; 32], A).expect("expected to register");
        assert!(proofs.is_registered(&[1; 32]));
        assert_eq!(proofs.get(&[1; 32]), None);

        proofs.store([1; 32], 5, proof_at(5));
        proofs.block_committed(5);

        assert!(!proofs.has_registered());
        assert_eq!(proofs.get(&[1; 32]).as_deref(), Some(&proof_at(5)));

        // The lease is held until its client releases it, and its release needs no memory
        assert_eq!(proofs.inner().registrations, 1);
        proofs.unregister(&[1; 32], &A);
        assert_eq!(proofs.inner().registrations, 0);
        assert!(proofs.inner().released.is_empty());
    }

    #[test]
    fn should_keep_a_transition_registered_while_another_lease_holds_it() {
        let proofs = StateTransitionExecutionProofs::default();

        proofs.register([1; 32], A).expect("expected to register");
        proofs.register([1; 32], B).expect("expected to register");
        assert_eq!(proofs.inner().registrations, 2);

        proofs.unregister(&[1; 32], &A);
        assert!(proofs.is_registered(&[1; 32]));

        proofs.unregister(&[1; 32], &B);
        assert!(!proofs.is_registered(&[1; 32]));
        assert_eq!(proofs.inner().registrations, 0);
    }

    #[test]
    fn should_take_and_release_a_lease_once() {
        let proofs = StateTransitionExecutionProofs::default();

        proofs.register([1; 32], A).expect("expected to register");
        proofs.register([1; 32], A).expect("expected to register");
        assert_eq!(proofs.inner().registrations, 1);

        proofs.unregister(&[1; 32], &A);
        proofs.unregister(&[1; 32], &A);
        assert_eq!(proofs.inner().registrations, 0);
    }

    #[test]
    fn should_not_take_a_registration_that_arrives_after_its_release() {
        let proofs = StateTransitionExecutionProofs::default();

        // The wait was cancelled while its registration was on its way
        proofs.unregister(&[1; 32], &A);
        assert_eq!(
            proofs.register([1; 32], A),
            Err(RegistrationRefused::LeaseReleased)
        );

        assert!(!proofs.is_registered(&[1; 32]));
        assert_eq!(proofs.inner().registrations, 0);
    }

    /// The lease of an issuer's lease with this sequence number.
    fn issued(issuer: u8, sequence: u64) -> RegistrationLease {
        let mut lease = [issuer; 16];
        lease[8..].copy_from_slice(&sequence.to_be_bytes());
        lease
    }

    #[test]
    fn should_keep_refusing_forgotten_late_registrations_and_take_fresh_ones() {
        let proofs = StateTransitionExecutionProofs::new(10);

        // Releases of completed waits need no memory, so they never fill the table
        for sequence in 0..2 * MAX_REMEMBERED_RELEASES as u64 {
            proofs
                .register([1; 32], issued(1, sequence))
                .expect("expected to register");
            proofs.unregister(&[1; 32], &issued(1, sequence));
        }
        assert!(proofs.inner().released.is_empty());

        // Cancelled waits whose registrations never arrived fill it, and one more overflows it
        let first = 10_000;
        for sequence in first..=first + MAX_REMEMBERED_RELEASES as u64 {
            proofs.unregister(&[1; 32], &issued(2, sequence));
        }
        assert_eq!(proofs.inner().released.len(), MAX_REMEMBERED_RELEASES);
        assert_eq!(proofs.inner().registrations, 0);

        // The forgotten release's late registration is still not taken, nor is a remembered one's
        assert_eq!(
            proofs.register([1; 32], issued(2, first)),
            Err(RegistrationRefused::LeaseReleased)
        );
        assert_eq!(
            proofs.register([1; 32], issued(2, first + 5)),
            Err(RegistrationRefused::LeaseReleased)
        );
        assert!(!proofs.has_registered());

        // A fresh lease of the same issuer, and one of another issuer, are taken
        assert_eq!(
            proofs.register(
                [1; 32],
                issued(2, first + 2 * MAX_REMEMBERED_RELEASES as u64)
            ),
            Ok(())
        );
        assert_eq!(proofs.register([2; 32], issued(3, 0)), Ok(()));
        assert_eq!(proofs.inner().registrations, 2);
    }

    #[test]
    fn should_tell_a_registration_below_its_issuers_low_water_mark_that_it_was_not_taken() {
        let proofs = StateTransitionExecutionProofs::new(10);

        // A wait's registration is still on its way while later waits of the same issuer are
        // cancelled before theirs arrive, until a release is forgotten and raises the mark
        let in_flight = issued(2, 5);
        for sequence in 10..=10 + MAX_REMEMBERED_RELEASES as u64 {
            proofs.unregister(&[1; 32], &issued(2, sequence));
        }

        // Its registration is refused rather than acknowledged, so the wait knows it holds
        // nothing and goes on without a pair
        assert_eq!(
            proofs.register([1; 32], in_flight),
            Err(RegistrationRefused::LeaseReleased)
        );
        assert!(!proofs.has_registered());
    }

    #[test]
    fn should_forget_a_low_water_mark_after_a_lifetime() {
        let proofs = StateTransitionExecutionProofs::new(10);
        for sequence in 0..=MAX_REMEMBERED_RELEASES as u64 {
            proofs.unregister(&[1; 32], &issued(2, sequence));
        }
        assert!(!proofs.inner().release_low_water_marks.is_empty());

        proofs.block_committed(10 + REGISTRATION_LIFETIME_BLOCKS + 1);

        assert!(proofs.inner().release_low_water_marks.is_empty());
        assert!(proofs.inner().released.is_empty());
    }

    #[test]
    fn should_end_only_its_own_registration_on_a_stale_release() {
        let proofs = StateTransitionExecutionProofs::default();

        proofs.register([1; 32], A).expect("expected to register");
        proofs.unregister(&[1; 32], &A);
        // Another wait for the same transition registers after the first one ended
        proofs.register([1; 32], B).expect("expected to register");

        // The first wait's release arrives again
        proofs.unregister(&[1; 32], &A);

        assert!(proofs.is_registered(&[1; 32]));
    }

    #[test]
    fn should_not_wait_for_a_proved_transition_and_keep_its_proof_when_released() {
        let proofs = StateTransitionExecutionProofs::default();

        proofs.register([1; 32], A).expect("expected to register");
        proofs.store([1; 32], 5, proof_at(5));

        // A wait that starts after the proof takes a lease, which its release then finds
        proofs.register([1; 32], B).expect("expected to register");
        assert!(!proofs.is_registered(&[1; 32]));

        proofs.unregister(&[1; 32], &A);
        proofs.unregister(&[1; 32], &B);
        assert_eq!(proofs.get(&[1; 32]).as_deref(), Some(&proof_at(5)));
        assert!(proofs.inner().released.is_empty());
    }

    #[test]
    fn should_refuse_a_registration_past_the_limit_until_one_is_released() {
        let proofs = StateTransitionExecutionProofs::default();

        for index in 0..MAX_REGISTERED_STATE_TRANSITIONS {
            proofs
                .register(hash_of(index), lease_of(index))
                .expect("expected to register");
        }
        assert_eq!(
            proofs.register([255; 32], [255; 16]),
            Err(RegistrationRefused::TooManyRegistered)
        );
        // A second lease on a registered transition is a registration too
        assert_eq!(
            proofs.register(hash_of(0), [255; 16]),
            Err(RegistrationRefused::TooManyRegistered)
        );

        proofs.unregister(&hash_of(0), &lease_of(0));

        assert_eq!(proofs.register([255; 32], [255; 16]), Ok(()));
    }

    #[test]
    fn should_expire_each_registration_on_its_own() {
        let proofs = StateTransitionExecutionProofs::new(10);
        proofs.register([1; 32], A).expect("expected to register");
        proofs.block_committed(20);
        proofs.register([1; 32], B).expect("expected to register");

        proofs.block_committed(10 + REGISTRATION_LIFETIME_BLOCKS + 1);
        assert!(proofs.is_registered(&[1; 32]));
        assert_eq!(proofs.inner().registrations, 1);

        proofs.block_committed(20 + REGISTRATION_LIFETIME_BLOCKS + 1);
        assert!(!proofs.is_registered(&[1; 32]));
        assert_eq!(proofs.inner().registrations, 0);
    }

    #[test]
    fn should_count_a_registration_lifetime_from_the_height_the_node_restarted_at() {
        // A node reopened at height 100 takes a registration before block 101
        let proofs = StateTransitionExecutionProofs::new(100);
        proofs.register([1; 32], A).expect("expected to register");

        // Block 101 does not hold the transition; it is still waiting for block 102
        proofs.block_committed(101);

        assert!(proofs.is_registered(&[1; 32]));
    }

    #[test]
    fn should_forget_a_proof_past_its_retention() {
        let proofs = StateTransitionExecutionProofs::default();
        proofs.store([1; 32], 10, proof_of_size(10, 100));
        assert_eq!(proofs.inner().proved_bytes, 200);

        proofs.block_committed(10 + PROOF_RETENTION_BLOCKS);
        assert!(proofs.get(&[1; 32]).is_some());

        proofs.block_committed(10 + PROOF_RETENTION_BLOCKS + 1);
        assert_eq!(proofs.get(&[1; 32]), None);
        assert_eq!(proofs.inner().proved_bytes, 0);
    }

    #[test]
    fn should_not_keep_a_pair_past_the_byte_budget() {
        let proofs = StateTransitionExecutionProofs::default();
        let half = MAX_RETAINED_PROOF_BYTES / 4;

        // Two pairs of two sides of a quarter each fill the budget
        proofs.store([1; 32], 10, proof_of_size(10, half));
        proofs.store([2; 32], 10, proof_of_size(10, half));
        proofs.register([3; 32], A).expect("expected to register");
        proofs.store([3; 32], 10, proof_of_size(10, 1));

        assert!(proofs.get(&[1; 32]).is_some());
        assert!(proofs.get(&[2; 32]).is_some());
        assert_eq!(proofs.get(&[3; 32]), None);
    }
}
