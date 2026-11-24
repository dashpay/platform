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
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use dpp::util::hash::hash_single;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// How many registrations can be held at once, one per waiting client. A registration past
/// this is refused.
///
/// It matches the number of waits one rs-dapi holds at once, so every wait of the rs-dapi in
/// front of this node can hold one.
pub const MAX_REGISTERED_STATE_TRANSITIONS: usize = 1024;

/// How many blocks a registration lives when nothing releases it, as when the client that took
/// it stopped without saying so. Each registration expires on its own.
pub const REGISTRATION_LIFETIME_BLOCKS: u64 = 64;

/// How long a registration lives when nothing releases it, however few blocks came since. On a
/// quiet chain [`REGISTRATION_LIFETIME_BLOCKS`] blocks take hours, and registrations whose
/// release never arrived, as when the rs-dapi that took them restarted, would hold their slots
/// all that time. It is longer than rs-dapi lets any wait run, so a registration a client still
/// waits on does not run out of it. It is counted on this node's clock, which like the rest of
/// the registry is outside consensus: it only decides which registrations the node still holds.
pub const REGISTRATION_LIFETIME: Duration = Duration::from_secs(15 * 60);

/// How many blocks a proof is kept for after the block it is of.
pub const PROOF_RETENTION_BLOCKS: u64 = 64;

/// How many registered transitions of one block are proved. Each costs two proofs while the
/// block is finalized, so this bounds what registrations add to finalizing a block, together
/// with proving only the kinds whose proof is a small fixed set of keys. The registered
/// transitions of a block past this are not proved, in block order.
pub const MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK: usize = 32;

/// How many registered transitions of one block are decoded to find the ones to prove, of any
/// kind. A block can hold registered transitions of kinds that are not proved, and each costs a
/// full decode before its kind is known; the registered transitions past this are not proved,
/// in block order.
pub const MAX_DECODED_STATE_TRANSITIONS_PER_BLOCK: usize =
    2 * MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK;

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
/// late registration under it is taken and expires as one nothing releases does.
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

/// When a registration was taken.
#[derive(Clone, Copy, Debug)]
struct Taken {
    /// The height of the last committed block then.
    height: u64,
    /// The time then, on this node's clock.
    at: Instant,
}

impl Taken {
    /// Whether the registration is within both of its lifetimes, with the last committed block
    /// at `height` and the time `now`.
    fn is_live(&self, height: u64, now: Instant) -> bool {
        height.saturating_sub(self.height) <= REGISTRATION_LIFETIME_BLOCKS
            && now.saturating_duration_since(self.at) <= REGISTRATION_LIFETIME
    }
}

#[derive(Debug, Default)]
struct Inner {
    /// The registered transitions: the registrations waiting for each, with when each was
    /// taken.
    registered: HashMap<StateTransitionHash, HashMap<RegistrationLease, Taken>>,
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

    /// Forgets the registrations that have outlived either of their lifetimes at the time
    /// `now`: [`REGISTRATION_LIFETIME_BLOCKS`] and [`REGISTRATION_LIFETIME`].
    fn expire_registrations(&mut self, now: Instant) {
        let height = self.last_committed_height;
        let mut expired = 0;
        self.registered.retain(|_, leases| {
            let held = leases.len();
            leases.retain(|_, taken| taken.is_live(height, now));
            expired += held - leases.len();
            !leases.is_empty()
        });
        self.registrations -= expired;
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
    /// holds nothing for it. A remembered release refuses its registration once and is then
    /// forgotten: no other registration of that lease follows. Releases are remembered within
    /// [`MAX_REMEMBERED_RELEASES`], [`MAX_RELEASE_ISSUERS`] and [`REGISTRATION_LIFETIME_BLOCKS`]:
    /// a registration that arrives after its release is forgotten is taken, and held until it
    /// expires. A lease is held until it is released or expires, also once its transition is
    /// proved, so that its release finds it. It expires after [`REGISTRATION_LIFETIME_BLOCKS`]
    /// blocks or after [`REGISTRATION_LIFETIME`], whichever comes first.
    pub fn register(
        &self,
        hash: StateTransitionHash,
        lease: RegistrationLease,
    ) -> Result<(), RegistrationRefused> {
        self.register_at(hash, lease, Instant::now())
    }

    /// [`register`](Self::register) at the time `now`.
    fn register_at(
        &self,
        hash: StateTransitionHash,
        lease: RegistrationLease,
        now: Instant,
    ) -> Result<(), RegistrationRefused> {
        let mut inner = self.inner();
        if inner.released.remove(&lease).is_some() || inner.below_low_water_mark(&lease) {
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
            // Blocks can be minutes apart, so the registrations that ran out of time since the
            // last one are forgotten here too, when their slots are needed
            inner.expire_registrations(now);
        }
        if inner.registrations >= MAX_REGISTERED_STATE_TRANSITIONS {
            return Err(RegistrationRefused::TooManyRegistered);
        }
        let taken = Taken {
            height: inner.last_committed_height,
            at: now,
        };
        inner
            .registered
            .entry(hash)
            .or_default()
            .insert(lease, taken);
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

    /// The hashes of the registered state transitions waiting for their block.
    pub fn waiting_hashes(&self) -> HashSet<StateTransitionHash> {
        self.waiting_hashes_at(Instant::now())
    }

    /// [`waiting_hashes`](Self::waiting_hashes) at the time `now`.
    fn waiting_hashes_at(&self, now: Instant) -> HashSet<StateTransitionHash> {
        let mut inner = self.inner();
        // A block can come long after the last one, so the registrations that ran out of time
        // since are forgotten first: a transition nobody waits for any more takes none of the
        // block's proving budget
        inner.expire_registrations(now);
        inner
            .registered
            .keys()
            .filter(|hash| inner.waits_for_block(hash))
            .copied()
            .collect()
    }

    /// Whether the state transition with this hash is waiting for its block.
    #[cfg(test)]
    pub fn is_registered(&self, hash: &StateTransitionHash) -> bool {
        self.inner().waits_for_block(hash)
    }

    /// Whether any state transition is waiting for its block.
    #[cfg(test)]
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

    /// Records that the block at `height` is committed and keeps the pairs `proved` of the
    /// transitions it executed. What has outlived its bounds is forgotten first, so the bytes
    /// of pairs that expire at this block make room for this block's own.
    pub(crate) fn commit_block(
        &self,
        height: u64,
        proved: Vec<(StateTransitionHash, StateTransitionExecutionProof)>,
    ) {
        self.block_committed(height);
        for (hash, proof) in proved {
            self.store(hash, height, proof);
        }
    }

    /// Records that the block at `height` is committed, and forgets the registrations, the
    /// releases and the proofs that have outlived their bounds.
    fn block_committed(&self, height: u64) {
        self.block_committed_at(height, Instant::now());
    }

    /// [`block_committed`](Self::block_committed) at the time `now`.
    fn block_committed_at(&self, height: u64, now: Instant) {
        let mut inner = self.inner();
        inner.last_committed_height = height;
        let live = |at: &u64| height.saturating_sub(*at) <= REGISTRATION_LIFETIME_BLOCKS;

        inner.expire_registrations(now);
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
    /// Only the kinds [`is_captured`] admits are proved, at most
    /// [`MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK`] of them in block order, and at most
    /// [`MAX_DECODED_STATE_TRANSITIONS_PER_BLOCK`] registered transitions are decoded to find
    /// them. A transition skipped for any of these reasons, or one that cannot be proved, stays
    /// registered until its client releases it.
    pub(crate) fn prove_registered_state_transitions_before_block(
        &self,
        raw_state_transitions: &[Vec<u8>],
    ) -> Vec<PendingStateTransitionExecutionProof> {
        // The registered transitions still waiting for their block, read under one lock, so
        // the scan below does not contend with registrations
        let waiting = self.state_transition_execution_proofs.waiting_hashes();
        if waiting.is_empty() {
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

        // One pass over the block that stops at whichever budget runs out first: at most
        // MAX_DECODED_STATE_TRANSITIONS_PER_BLOCK registered transitions are decoded, whatever
        // their kind, and at most MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK are proved.
        let mut decoded = 0;
        let mut provable = Vec::new();
        for raw_state_transition in raw_state_transitions {
            if decoded == MAX_DECODED_STATE_TRANSITIONS_PER_BLOCK
                || provable.len() == MAX_PROVED_STATE_TRANSITIONS_PER_BLOCK
            {
                tracing::debug!(
                    decoded,
                    proved = provable.len(),
                    "the block's budget for registered state transitions is spent; \
                     the rest of the block is not proved"
                );
                break;
            }
            let hash = hash_single(raw_state_transition);
            if !waiting.contains(&hash) {
                continue;
            }
            decoded += 1;
            let state_transition =
                match StateTransition::deserialize_from_bytes_untrusted(raw_state_transition) {
                    Ok(state_transition) => state_transition,
                    Err(error) => {
                        tracing::warn!(
                            state_transition_hash = hex::encode(hash),
                            ?error,
                            "cannot decode a registered state transition to prove it"
                        );
                        continue;
                    }
                };
            if !is_captured(&state_transition, platform_version) {
                tracing::debug!(
                    state_transition_hash = hex::encode(hash),
                    "a registered state transition of a kind that is not proved"
                );
                continue;
            }
            provable.push((hash, state_transition));
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
        let mut proved = Vec::new();
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
                    proved.push((hash, StateTransitionExecutionProof { before, after }));
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
            .commit_block(height, proved);
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

        // The kind is admitted again with this side's own version: across an upgrade the block
        // after may prove it differently
        if !is_captured(state_transition, platform_version) {
            tracing::debug!(
                state_transition_hash = hex::encode(hash),
                "a registered state transition of a kind that is not proved at this block's version"
            );
            return None;
        }

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

        // A client refuses a GroveDB proof envelope below its verifier's floor, and with it the
        // whole response, so a side below the floor is not made, and without it no pair is kept.
        // Each side is checked on its own: two sides of different envelope versions both above
        // the floor make a pair. A block at protocol version 11 or earlier (GroveDB version 2,
        // envelope V0) is below it, so the first block after the upgrade to 12 keeps no pair
        // and its wait answers with a proof of the current state.
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

        // Signed off and described by the generation of the block whose state it is
        let (proof, metadata) = self
            .committed_block_proof_and_metadata(platform_state, grovedb_proof, platform_version)
            .inspect_err(|error| {
                tracing::warn!(
                    state_transition_hash = hex::encode(hash),
                    ?error,
                    "failed to sign off the proof of a registered state transition"
                )
            })
            .ok()?;

        Some(ProofOfStateAtBlock { proof, metadata })
    }
}

/// Whether a state transition's state before and after its block is proved at this platform
/// version: a client can read the state before it, and the prover declares its proof to be of
/// a fixed set of keys, so that making it while a block is finalized costs a bounded amount of
/// work. Each side is checked with the version of the block it is of.
fn is_captured(state_transition: &StateTransition, platform_version: &PlatformVersion) -> bool {
    Drive::state_transition_prior_state_is_readable(state_transition, platform_version)
        .unwrap_or(false)
        && Drive::state_transition_proof_has_fixed_keys(state_transition, platform_version)
            .unwrap_or(false)
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
    fn should_forget_an_early_release_once_it_has_refused_its_registration() {
        let proofs = StateTransitionExecutionProofs::new(10);

        // A wait's registration is still on its way while more later waits than the table
        // holds are cancelled, each release followed by its late registration
        let in_flight = issued(2, 5);
        for sequence in 10..10 + 2 * MAX_REMEMBERED_RELEASES as u64 {
            proofs.unregister(&[1; 32], &issued(2, sequence));
            assert_eq!(
                proofs.register([1; 32], issued(2, sequence)),
                Err(RegistrationRefused::LeaseReleased)
            );
        }
        // They never fill the table nor raise the issuer's mark
        assert!(proofs.inner().released.is_empty());
        assert!(proofs.inner().release_low_water_marks.is_empty());

        // So the registration still on its way is taken when it arrives
        assert_eq!(proofs.register([1; 32], in_flight), Ok(()));
    }

    #[test]
    fn should_take_a_late_registration_once_its_issuers_mark_is_forgotten() {
        let proofs = StateTransitionExecutionProofs::new(10);
        // One issuer's releases fill the table
        for sequence in 1_000..1_000 + MAX_REMEMBERED_RELEASES as u64 {
            proofs.unregister(&[1; 32], &issued(200, sequence));
        }

        // Each early release of another issuer is the lowest, so it is forgotten at once and
        // raises its issuer's mark: issuer 0's at block 10, the next ones' at block 11
        proofs.unregister(&[1; 32], &issued(0, 0));
        proofs.block_committed(11);
        for issuer in 1..=MAX_RELEASE_ISSUERS as u8 {
            proofs.unregister(&[1; 32], &issued(issuer, u64::from(issuer)));
        }
        assert_eq!(
            proofs.inner().release_low_water_marks.len(),
            MAX_RELEASE_ISSUERS
        );

        // Issuer 0's mark was the oldest when one more issuer needed one, so its late
        // registration is taken, while a kept mark still refuses its own
        assert_eq!(proofs.register([1; 32], issued(0, 0)), Ok(()));
        assert_eq!(
            proofs.register([1; 32], issued(1, 1)),
            Err(RegistrationRefused::LeaseReleased)
        );
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
    fn should_expire_a_registration_by_time_when_blocks_are_slow_to_come() {
        let proofs = StateTransitionExecutionProofs::new(10);
        let taken_at = Instant::now();
        proofs
            .register_at([1; 32], A, taken_at)
            .expect("expected to register");

        // Far fewer blocks than a lifetime in blocks, each minutes after the last
        proofs.block_committed_at(11, taken_at + REGISTRATION_LIFETIME);
        assert!(proofs.is_registered(&[1; 32]));

        proofs.block_committed_at(
            12,
            taken_at + REGISTRATION_LIFETIME + Duration::from_secs(1),
        );
        assert!(!proofs.is_registered(&[1; 32]));
        assert_eq!(proofs.inner().registrations, 0);
    }

    #[test]
    fn should_give_the_slots_of_timed_out_registrations_to_new_ones_between_blocks() {
        let proofs = StateTransitionExecutionProofs::new(10);
        let taken_at = Instant::now();
        // Registrations whose release never arrives, as of an rs-dapi that restarted
        for index in 0..MAX_REGISTERED_STATE_TRANSITIONS {
            proofs
                .register_at(hash_of(index), lease_of(index), taken_at)
                .expect("expected to register");
        }

        // Within their lifetime they hold every slot
        assert_eq!(
            proofs.register_at([255; 32], [255; 16], taken_at + REGISTRATION_LIFETIME),
            Err(RegistrationRefused::TooManyRegistered)
        );

        // Past it a new registration takes a slot, with no block committed in between
        let later = taken_at + REGISTRATION_LIFETIME + Duration::from_secs(1);
        assert_eq!(proofs.register_at([255; 32], [255; 16], later), Ok(()));
        assert_eq!(proofs.inner().registrations, 1);
        assert!(!proofs.is_registered(&hash_of(0)));
    }

    #[test]
    fn should_not_wait_for_a_transition_whose_registrations_ran_out_of_time_since_the_last_block() {
        let proofs = StateTransitionExecutionProofs::new(10);
        let taken_at = Instant::now();
        proofs
            .register_at([1; 32], A, taken_at)
            .expect("expected to register");
        proofs
            .register_at([2; 32], B, taken_at)
            .expect("expected to register");
        // A wait that started later still waits for the second transition
        let later = taken_at + REGISTRATION_LIFETIME;
        proofs
            .register_at([2; 32], lease_of(3), later)
            .expect("expected to register");

        // The next block comes once the first two registrations ran out of time, with no block
        // committed and no full registry to forget them before
        let waiting = proofs.waiting_hashes_at(later + Duration::from_secs(1));

        assert_eq!(waiting, HashSet::from([[2; 32]]));
        assert_eq!(proofs.inner().registrations, 1);
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
    fn should_keep_a_pair_of_the_block_that_frees_the_bytes_it_needs() {
        let proofs = StateTransitionExecutionProofs::default();
        let quarter = MAX_RETAINED_PROOF_BYTES / 4;
        proofs.store([1; 32], 10, proof_of_size(10, quarter));
        proofs.store([2; 32], 10, proof_of_size(10, quarter));

        // The pairs of block 10 expire at the block whose own pair needs their bytes
        let height = 10 + PROOF_RETENTION_BLOCKS + 1;
        proofs.commit_block(height, vec![([3; 32], proof_of_size(height, quarter))]);

        assert_eq!(proofs.get(&[1; 32]), None);
        assert_eq!(proofs.get(&[2; 32]), None);
        assert!(proofs.get(&[3; 32]).is_some());
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
