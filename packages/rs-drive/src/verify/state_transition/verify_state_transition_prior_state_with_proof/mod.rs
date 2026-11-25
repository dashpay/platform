//! Verifies the state a state transition's keys held before the transition executed.
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

mod v0;

impl Drive {
    /// Whether [`Self::verify_state_transition_prior_state_with_proof`] reads the state before
    /// this state transition, decided from the transition alone.
    ///
    /// The kinds it reads are the ones whose proof is a small, fixed set of keys that exist
    /// before and after the transition alike, and whose reader checks nothing against what the
    /// transition leaves: credit transfers, top-ups and credit withdrawals of an identity.
    ///
    /// # Parameters
    ///
    /// - `state_transition`: the state transition.
    /// - `platform_version`: the caller's version, which selects the generation.
    ///
    /// # Returns
    ///
    /// Whether the state before the transition is read, or an error for a generation this build
    /// does not know.
    pub fn state_transition_prior_state_is_readable(
        state_transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_prior_state_with_proof
        {
            0 => Ok(Self::state_transition_prior_state_is_readable_v0(
                state_transition,
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "state_transition_prior_state_is_readable".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Verifies a proof of a state transition's keys made at a block before the transition
    /// executed, and reads their state there.
    ///
    /// The proof is the one `prove_state_transition` makes for the transition, so the keys are
    /// the same ones the proof of the state after it holds. Unlike
    /// `verify_state_transition_was_executed_with_proof`, nothing is checked against what the
    /// transition leaves, and a key that does not exist yet is read as absent rather than
    /// refused: an identity created in the same block as a transfer to it has no balance before.
    ///
    /// # Parameters
    ///
    /// - `state_transition`: the state transition the proof was made for.
    /// - `proof`: the GroveDB proof.
    /// - `platform_version`: the caller's version, which selects the generation.
    ///
    /// # Returns
    ///
    /// `None` for a kind of state transition whose state before it is not read (see
    /// [`Self::state_transition_prior_state_is_readable`]), without reading the proof.
    /// Otherwise the root hash of the proof and the state it holds:
    /// `VerifiedBalanceTransfer` for a credit transfer and `VerifiedPartialIdentity` for a
    /// top-up or a credit withdrawal, with `None` for a balance that does not exist.
    ///
    /// # Errors
    ///
    /// A proof that does not verify for the transition's keys, or a generation this build does
    /// not know.
    pub fn verify_state_transition_prior_state_with_proof(
        state_transition: &StateTransition,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Option<(RootHash, StateTransitionProofResult)>, Error> {
        match platform_version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_prior_state_with_proof
        {
            0 => Self::verify_state_transition_prior_state_with_proof_v0(
                state_transition,
                proof,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_state_transition_prior_state_with_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
