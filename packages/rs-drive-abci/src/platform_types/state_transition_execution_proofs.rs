//! Proofs of the state before and after the block that executed a state transition.
//!
//! A proof of a state transition's keys shows the state at the block the proof was made at.
//! Made when a client asks, it is of whatever block the node has reached by then. To show
//! what the transition's block changed, two proofs of the same keys are needed: one of the
//! state the previous block left, and one of the state the transition's own block left.
//!
//! The first can only be made before the transition's block is committed, so a node has to
//! know in advance which transitions to prove. A client registers a transition by its hash.
//! When a block that holds a registered transition is finalized, the node proves the
//! transition's keys before it commits the block and again after, and keeps the pair for the
//! client to fetch.
//!
//! None of this is consensus. It reads committed state, writes nothing to it, and a failure
//! to prove never fails a block. The bounds below are this node's own memory limits.

use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::query::response_metadata::CheckpointUsed;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::state_transition::StateTransition;
use dpp::util::hash::hash_single;
use drive::util::grove_operations::GroveDBToUse;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// How many transitions can be registered at once. A registration past this is refused.
pub const MAX_REGISTERED_STATE_TRANSITIONS: usize = 1024;

/// How many blocks a registration waits for its transition. A transition that no block held
/// by then is forgotten.
pub const REGISTRATION_LIFETIME_BLOCKS: u64 = 64;

/// How many blocks a proof is kept for after the block it is of.
pub const PROOF_RETENTION_BLOCKS: u64 = 64;

/// The hash a state transition is registered and fetched by: the hash of its bytes, which is
/// also its transaction hash in a block.
pub type StateTransitionHash = [u8; 32];

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
    /// [`MAX_REGISTERED_STATE_TRANSITIONS`] transitions are registered already.
    TooManyRegistered,
}

#[derive(Debug, Default)]
struct Inner {
    /// The registered transitions, with the height of the last committed block when each was
    /// registered.
    registered: HashMap<StateTransitionHash, u64>,
    /// The proofs made, with the height of the block each transition was executed in.
    proved: HashMap<StateTransitionHash, (u64, Arc<StateTransitionExecutionProof>)>,
    /// The height of the last block this node committed while running.
    last_committed_height: u64,
}

/// The state transitions a node was asked to prove, and the proofs it has made of them.
#[derive(Debug, Default)]
pub struct StateTransitionExecutionProofs {
    inner: Mutex<Inner>,
}

impl StateTransitionExecutionProofs {
    /// A panic while the lock is held leaves the maps as they were between two operations, each
    /// of which leaves them consistent, so a poisoned lock is still safe to use.
    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Asks the node to prove the state before and after the block that executes the state
    /// transition with this hash. Registering a transition again, or one already proved,
    /// changes nothing.
    pub fn register(&self, hash: StateTransitionHash) -> Result<(), RegistrationRefused> {
        let mut inner = self.inner();
        if inner.registered.contains_key(&hash) || inner.proved.contains_key(&hash) {
            return Ok(());
        }
        if inner.registered.len() >= MAX_REGISTERED_STATE_TRANSITIONS {
            return Err(RegistrationRefused::TooManyRegistered);
        }
        let registered_at = inner.last_committed_height;
        inner.registered.insert(hash, registered_at);
        Ok(())
    }

    /// Whether the state transition with this hash is waiting for its block.
    pub fn is_registered(&self, hash: &StateTransitionHash) -> bool {
        self.inner().registered.contains_key(hash)
    }

    /// Whether any state transition is waiting for its block.
    pub fn has_registered(&self) -> bool {
        !self.inner().registered.is_empty()
    }

    /// The proof of the state transition with this hash, if the node made one and still keeps
    /// it.
    pub fn get(&self, hash: &StateTransitionHash) -> Option<Arc<StateTransitionExecutionProof>> {
        self.inner()
            .proved
            .get(hash)
            .map(|(_, proof)| Arc::clone(proof))
    }

    /// Keeps the proof of a state transition executed in the block at `height`. The transition
    /// is no longer waiting.
    fn store(&self, hash: StateTransitionHash, height: u64, proof: StateTransitionExecutionProof) {
        let mut inner = self.inner();
        inner.registered.remove(&hash);
        inner.proved.insert(hash, (height, Arc::new(proof)));
    }

    /// Records that the block at `height` is committed, and forgets the registrations and the
    /// proofs that have outlived their bounds.
    fn block_committed(&self, height: u64) {
        let mut inner = self.inner();
        inner.last_committed_height = height;
        inner.registered.retain(|_, registered_at| {
            height.saturating_sub(*registered_at) <= REGISTRATION_LIFETIME_BLOCKS
        });
        inner.proved.retain(|_, (proved_at, _)| {
            height.saturating_sub(*proved_at) <= PROOF_RETENTION_BLOCKS
        });
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
    /// A transition that cannot be proved is skipped and stays registered.
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

        raw_state_transitions
            .iter()
            .filter_map(|raw_state_transition| {
                let hash = hash_single(raw_state_transition);
                if !self.state_transition_execution_proofs.is_registered(&hash) {
                    return None;
                }
                let state_transition =
                    match StateTransition::deserialize_from_bytes_untrusted(raw_state_transition) {
                        Ok(state_transition) => state_transition,
                        Err(error) => {
                            tracing::warn!(
                                state_transition_hash = hex::encode(hash),
                                ?error,
                                "cannot decode a registered state transition to prove it"
                            );
                            return None;
                        }
                    };
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

    fn proof_at(height: u64) -> StateTransitionExecutionProof {
        let side = |height| ProofOfStateAtBlock {
            proof: Proof::default(),
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

    #[test]
    fn should_keep_a_registered_transition_until_it_is_proved() {
        let proofs = StateTransitionExecutionProofs::default();
        assert!(!proofs.has_registered());

        proofs.register([1; 32]).expect("expected to register");
        assert!(proofs.is_registered(&[1; 32]));
        assert!(proofs.has_registered());
        assert_eq!(proofs.get(&[1; 32]), None);

        proofs.store([1; 32], 5, proof_at(5));
        proofs.block_committed(5);

        assert!(!proofs.is_registered(&[1; 32]));
        assert!(!proofs.has_registered());
        assert_eq!(proofs.get(&[1; 32]).as_deref(), Some(&proof_at(5)));
    }

    #[test]
    fn should_not_register_again_a_transition_that_is_registered_or_proved() {
        let proofs = StateTransitionExecutionProofs::default();

        proofs.register([1; 32]).expect("expected to register");
        proofs.register([1; 32]).expect("expected to register");
        assert_eq!(proofs.inner().registered.len(), 1);

        proofs.store([1; 32], 5, proof_at(5));
        proofs.register([1; 32]).expect("expected to register");
        assert!(!proofs.is_registered(&[1; 32]));
    }

    #[test]
    fn should_refuse_a_registration_past_the_limit() {
        let proofs = StateTransitionExecutionProofs::default();

        for index in 0..MAX_REGISTERED_STATE_TRANSITIONS {
            let mut hash = [0; 32];
            hash[..8].copy_from_slice(&(index as u64).to_be_bytes());
            proofs.register(hash).expect("expected to register");
        }

        assert_eq!(
            proofs.register([255; 32]),
            Err(RegistrationRefused::TooManyRegistered)
        );
        // One that is registered already is not a new registration
        assert_eq!(proofs.register([0; 32]), Ok(()));
    }

    #[test]
    fn should_forget_a_registration_no_block_held_in_time() {
        let proofs = StateTransitionExecutionProofs::default();
        proofs.block_committed(10);
        proofs.register([1; 32]).expect("expected to register");

        proofs.block_committed(10 + REGISTRATION_LIFETIME_BLOCKS);
        assert!(proofs.is_registered(&[1; 32]));

        proofs.block_committed(10 + REGISTRATION_LIFETIME_BLOCKS + 1);
        assert!(!proofs.is_registered(&[1; 32]));
    }

    #[test]
    fn should_forget_a_proof_past_its_retention() {
        let proofs = StateTransitionExecutionProofs::default();
        proofs.store([1; 32], 10, proof_at(10));

        proofs.block_committed(10 + PROOF_RETENTION_BLOCKS);
        assert!(proofs.get(&[1; 32]).is_some());

        proofs.block_committed(10 + PROOF_RETENTION_BLOCKS + 1);
        assert_eq!(proofs.get(&[1; 32]), None);
    }
}
