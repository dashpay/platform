//! The proven outcome of a state transition: it can no longer execute, it never executed and
//! never will, or it may still execute and until when.
//!
//! [`Sdk::state_transition_outcome`] reads the transition's inputs with proofs the SDK verifies:
//! the shielded pools' anchors and nullifiers for a shielded spend, the nonces for a transition
//! that takes them. It needs no new query and no Drive change. A client that broadcast a
//! transition and heard nothing clear back uses it to decide when to stop waiting, when to free
//! what it held back for the transition, and when to retry.
//!
//! [`NodeTransitionStatus`](super::node_status::NodeTransitionStatus) is the other half: what one
//! node says it saw, unproved. The two are separate types so that nothing unproved can stand in
//! for a proven outcome.
//!
//! # Privacy
//!
//! The reads name the transition's nullifiers or nonces, so a node that answers them learns that
//! the asker cares about that transition. For shielded and private document transitions, ask
//! only the node the transition was broadcast to (an [`Sdk`] whose address list holds only that
//! node): it already holds the transition, and asking several nodes spreads the link between the
//! client's network address and the transition.

mod inputs;
mod judge;

use crate::error::StaleNodeError;
use crate::platform::{Fetch, FetchMany};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::ResponseMetadata;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::identifier::Identifier;
use dpp::identity::identity_nonce::{validate_identity_nonce_update, MergeIdentityNonceResult};
use dpp::prelude::IdentityNonce;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive_proof_verifier::types::{
    AddressInfo, IdentityContractNonceFetcher, IdentityNonceFetcher, MostRecentShieldedAnchor,
    NoParamQuery, ShieldedAnchors, ShieldedNullifierStatuses, ShieldedNullifiersQuery,
    TokenShieldedNullifiersQuery, TokenShieldedPoolQuery,
};
use inputs::{NonceInput, PoolSpend, ShieldedPool, TransitionInputs};
use judge::{anchor_deadline, judge_nonces, judge_shielded, AnchorRead, InputRead, InputsVerdict};
use std::collections::{BTreeMap, BTreeSet};

/// How many times the nullifiers are read again when every answer is older than the anchor set
/// that misses the spend's anchor, before the call gives up with [`Error::StaleNode`].
const NULLIFIER_REREADS: usize = 2;

/// The outcome of a state transition, from reads of its inputs whose proofs the SDK verified.
///
/// # This is the only status that may drive fund or privacy decisions
///
/// Free a reservation, unlock a held contact pair or retry a payment only on
/// [`TransitionOutcome::CannotExecute`]: it is the one answer that proves the transition did not
/// execute and never will. Treat [`TransitionOutcome::ExecutedOrSuperseded`] as settled and never
/// retry on it: it may be this very transition. [`TransitionOutcome::StillPossible`] means keep
/// waiting.
///
/// A node's own view
/// ([`NodeTransitionStatus`](super::node_status::NodeTransitionStatus), from
/// `getRecentStateTransitionStatus`) is unproved and may be wrong or stale. It is for display and
/// for deciding whether waiting longer makes sense, never for any of the decisions above.
///
/// # What is not here
///
/// No answer says this transition executed. The reads show which inputs are taken, not who took
/// them: every note a shielded spend uses can be spent by other transitions (one replacing it,
/// or several spending the notes separately), a nonce is taken by whichever transition carries
/// it, and the node reports a committed error without a proof. All of these are reported as
/// [`TransitionOutcome::ExecutedOrSuperseded`]; a wallet that knows it spent the notes or used
/// the nonce nowhere else can read that answer as executed.
///
/// No answer says a transition is invalid either. One that fails today's structure checks may
/// pass a later protocol version's, so it stays [`TransitionOutcome::StillPossible`] while its
/// inputs are free; a shielded spend is still bounded by its anchor's deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionOutcome {
    /// It can no longer execute, and the proofs do not show whether it did: every input it takes
    /// is consumed, by this transition (executed, or failed with a paid error) or by others.
    /// For a shielded spend every nullifier is spent; for a transition that takes nonces every
    /// nonce is used.
    ///
    /// Never retry or release anything on this answer: it may be this very transition.
    ExecutedOrSuperseded,
    /// It did not execute and never will, for the given reason.
    CannotExecute(CannotExecuteReason),
    /// It had not executed when last read, and it may still execute.
    StillPossible {
        /// The height of the latest read showing every input still free.
        checked_at_height: u64,
        /// For a shielded spend whose anchors these reads found recorded: a height by which every
        /// one of them was recorded. Pass it back as [`OutcomeEvidence::anchors_recorded_by`] on
        /// a later call, so that an anchor missing then can be told pruned rather than not yet
        /// reached by the answering node. `None` otherwise.
        anchors_recorded_at: Option<u64>,
        /// The last height at which it can execute, under the anchor retention rules of the
        /// protocol version the reads were proven at, or `None` when nothing bounds it: a
        /// transition that takes nonces, or a shielded spend whose anchor is its pool's newest
        /// (pruning keeps the newest anchor while the pool is idle).
        ///
        /// A later protocol version may keep anchors longer, so past this height ask again for
        /// [`TransitionOutcome::CannotExecute`] rather than assuming it.
        until_height: Option<u64>,
    },
}

impl TransitionOutcome {
    /// Whether the proofs show the transition did not execute and never will: the one answer
    /// on which a client may free what it held back for the transition, or retry it.
    pub fn proves_not_executed(&self) -> bool {
        matches!(self, TransitionOutcome::CannotExecute(_))
    }

    /// Whether the transition can no longer change state: every answer but
    /// [`TransitionOutcome::StillPossible`].
    pub fn is_final(&self) -> bool {
        !matches!(self, TransitionOutcome::StillPossible { .. })
    }
}

/// Why a transition can never execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CannotExecuteReason {
    /// A shielded spend's anchor is gone from its pool's anchor set, at a height at or above one
    /// by which the caller's evidence ([`OutcomeEvidence::anchors_recorded_by`]) shows it was
    /// recorded, and the spend's nullifiers were unspent at or after that read.
    ///
    /// Platform accepts a spend only while its anchor is kept
    /// (`shielded_anchor_retention_blocks`, pruned every `shielded_anchor_pruning_interval`
    /// blocks), and a pruned anchor never returns, so the spend could only have executed before
    /// the anchor was pruned, which the unspent nullifiers rule out. Without the evidence an
    /// anchor missing from the set proves nothing: the answering node may simply not have
    /// reached the block that records it.
    AnchorExpired,
    /// Another transition took one of its inputs while another input was still free: a nullifier
    /// spent while another was unspent, or one nonce used while another was free. A transition
    /// takes all of its inputs at once, so this one did not take that input, and cannot execute
    /// without it.
    InputUsed,
}

/// What the caller already knows, from proofs it verified, that the reads alone cannot show.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutcomeEvidence {
    /// A height by which every anchor a shielded spend uses was recorded: for a wallet, the
    /// height of the synced commitment tree it took the anchor from (the anchor is that tree's
    /// root, which Platform records at the end of that block at the latest), or the
    /// `anchors_recorded_at` of an earlier [`TransitionOutcome::StillPossible`] for the same
    /// transition.
    ///
    /// Without it a shielded spend is never reported [`CannotExecuteReason::AnchorExpired`].
    pub anchors_recorded_by: Option<u64>,
}

impl OutcomeEvidence {
    /// Evidence that every anchor the spend uses was recorded by `height`.
    pub fn anchors_recorded_by(height: u64) -> Self {
        Self {
            anchors_recorded_by: Some(height),
        }
    }
}

impl Sdk {
    /// The proven outcome of `transition`: whether it can no longer execute, never executed and
    /// never will, or may still execute and until when. See [`TransitionOutcome`] for which
    /// answers may drive which decisions.
    ///
    /// Takes the whole transition, not its hash: the answer comes from its inputs. A shielded
    /// spend is judged by its pools' anchor sets, read first, then its nullifiers, read after:
    /// an anchor set read after the nullifiers could miss a spend that executed just before its
    /// anchor was pruned. `evidence` says by which height its anchors were recorded, without
    /// which a missing anchor proves nothing (see [`OutcomeEvidence`]). A transition that takes
    /// nonces is judged by them. A transition funded by an asset lock, and a masternode vote,
    /// are refused with [`Error::InvalidRequest`].
    ///
    /// Needs an SDK that verifies proofs. Ask only the node the transition was broadcast to for
    /// a shielded or private document transition (see the module documentation).
    pub async fn state_transition_outcome(
        &self,
        transition: &StateTransition,
        evidence: OutcomeEvidence,
    ) -> Result<TransitionOutcome, Error> {
        if !self.prove() {
            return Err(Error::Config(
                "a proven transition outcome needs an SDK that verifies proofs".to_string(),
            ));
        }
        let reads = match TransitionInputs::of(transition)? {
            TransitionInputs::Shielded(spends) => {
                self.read_shielded_spend(&spends, evidence.anchors_recorded_by)
                    .await?
            }
            TransitionInputs::Nonces(nonces) => self.read_nonces(&nonces).await?,
        };
        Ok(match reads.verdict {
            InputsVerdict::ConsumedUnattributed => TransitionOutcome::ExecutedOrSuperseded,
            InputsVerdict::InputUsed => {
                TransitionOutcome::CannotExecute(CannotExecuteReason::InputUsed)
            }
            InputsVerdict::AnchorExpired => {
                TransitionOutcome::CannotExecute(CannotExecuteReason::AnchorExpired)
            }
            InputsVerdict::NotYet { checked_at } => TransitionOutcome::StillPossible {
                checked_at_height: checked_at,
                anchors_recorded_at: reads.anchors_recorded_at,
                until_height: reads.until_height,
            },
            InputsVerdict::Unordered => {
                return Err(Error::InvalidProvedResponse(
                    "the nullifier reads could not be ordered after the anchor reads".to_string(),
                ));
            }
        })
    }

    /// Read a shielded spend: every pool's anchor set first, then the nullifiers, then, while
    /// it may still execute, each pool's newest anchor for the deadline.
    async fn read_shielded_spend(
        &self,
        spends: &[PoolSpend],
        anchors_recorded_by: Option<u64>,
    ) -> Result<Reads, Error> {
        let mut anchors = Vec::with_capacity(spends.len());
        for spend in spends {
            let (set, metadata) = self.fetch_pool_anchors(spend.pool).await?;
            anchors.push(AnchorRead {
                recorded: set.contains(&spend.anchor),
                height: metadata.height,
            });
        }

        let mut rereads = 0;
        let verdict = loop {
            let mut nullifiers = Vec::new();
            for spend in spends {
                let (spent, metadata) = self
                    .fetch_pool_nullifiers(spend.pool, &spend.nullifiers)
                    .await?;
                nullifiers.extend(spent.into_iter().map(|consumed| InputRead {
                    consumed,
                    height: metadata.height,
                }));
            }
            match judge_shielded(&nullifiers, &anchors, anchors_recorded_by) {
                InputsVerdict::Unordered if rereads < NULLIFIER_REREADS => rereads += 1,
                InputsVerdict::Unordered => {
                    return Err(StaleNodeError::Height {
                        expected_height: anchors.iter().map(|a| a.height).max().unwrap_or(0),
                        received_height: nullifiers.iter().map(|n| n.height).max().unwrap_or(0),
                        tolerance_blocks: 0,
                    }
                    .into());
                }
                verdict => break verdict,
            }
        };

        // A deadline, and evidence for a later call, only when every anchor was found recorded:
        // a missing anchor not proven pruned may belong to a block the node has not reached.
        let all_recorded = anchors.iter().all(|anchor| anchor.recorded);
        let (until_height, anchors_recorded_at) =
            if matches!(verdict, InputsVerdict::NotYet { .. }) && all_recorded {
                let mut until_height: Option<u64> = None;
                for (spend, anchor) in spends.iter().zip(&anchors) {
                    if let Some(deadline) = self.anchor_deadline(spend, anchor).await? {
                        until_height = Some(until_height.map_or(deadline, |u| u.min(deadline)));
                    }
                }
                // Each anchor was recorded by the height of the read that found it, so all of
                // them by the highest such height.
                let recorded_at = anchors.iter().map(|anchor| anchor.height).max();
                (until_height, recorded_at)
            } else {
                (None, None)
            };

        Ok(Reads {
            verdict,
            until_height,
            anchors_recorded_at,
        })
    }

    /// The last height at which `spend`'s anchor is kept, or `None` when nothing proves one.
    ///
    /// The deadline holds only for an anchor that is not its pool's newest, proven by a newest
    /// anchor read at or after the read that showed this anchor recorded, so this anchor was
    /// already recorded when the newer one was the newest.
    async fn anchor_deadline(
        &self,
        spend: &PoolSpend,
        anchor: &AnchorRead,
    ) -> Result<Option<u64>, Error> {
        let (newest, metadata) = self.fetch_pool_newest_anchor(spend.pool).await?;
        let newer_anchor_exists =
            metadata.height >= anchor.height && newest.is_some_and(|newest| newest != spend.anchor);
        let Ok(platform_version) = PlatformVersion::get(metadata.protocol_version) else {
            return Ok(None);
        };
        let constants = &platform_version
            .drive_abci
            .validation_and_processing
            .event_constants;
        Ok(anchor_deadline(
            anchor.height,
            newer_anchor_exists,
            constants.shielded_anchor_retention_blocks,
            constants.shielded_anchor_pruning_interval,
        ))
    }

    /// The anchors `pool` keeps, as a proven set. A pool with no anchor set keeps none.
    async fn fetch_pool_anchors(
        &self,
        pool: ShieldedPool,
    ) -> Result<(BTreeSet<[u8; 32]>, ResponseMetadata), Error> {
        let (anchors, metadata) = match pool {
            ShieldedPool::Credit => {
                ShieldedAnchors::fetch_with_metadata(self, NoParamQuery {}, None).await?
            }
            ShieldedPool::Token(token_id) => {
                ShieldedAnchors::fetch_with_metadata(
                    self,
                    TokenShieldedPoolQuery { token_id },
                    None,
                )
                .await?
            }
        };
        let anchors = anchors
            .map(|ShieldedAnchors(anchors)| anchors.into_iter().collect())
            .unwrap_or_default();
        Ok((anchors, metadata))
    }

    /// Whether each of `nullifiers` is spent in `pool`, in the order given.
    async fn fetch_pool_nullifiers(
        &self,
        pool: ShieldedPool,
        nullifiers: &[[u8; 32]],
    ) -> Result<(Vec<bool>, ResponseMetadata), Error> {
        let (statuses, metadata) = match pool {
            ShieldedPool::Credit => {
                ShieldedNullifierStatuses::fetch_with_metadata(
                    self,
                    ShieldedNullifiersQuery(nullifiers.to_vec()),
                    None,
                )
                .await?
            }
            ShieldedPool::Token(token_id) => {
                ShieldedNullifierStatuses::fetch_with_metadata(
                    self,
                    TokenShieldedNullifiersQuery {
                        token_id,
                        nullifiers: nullifiers.to_vec(),
                    },
                    None,
                )
                .await?
            }
        };
        let spent: BTreeMap<[u8; 32], bool> = statuses
            .map(|ShieldedNullifierStatuses(statuses)| {
                statuses
                    .into_iter()
                    .map(|status| (status.nullifier, status.is_spent))
                    .collect()
            })
            .unwrap_or_default();
        let spent = nullifiers
            .iter()
            .map(|nullifier| {
                spent.get(nullifier).copied().ok_or_else(|| {
                    Error::InvalidProvedResponse(format!(
                        "the proven nullifier statuses leave out nullifier {}",
                        hex::encode(nullifier)
                    ))
                })
            })
            .collect::<Result<Vec<bool>, Error>>()?;
        Ok((spent, metadata))
    }

    /// `pool`'s newest anchor, proven, or `None` when it has none.
    async fn fetch_pool_newest_anchor(
        &self,
        pool: ShieldedPool,
    ) -> Result<(Option<[u8; 32]>, ResponseMetadata), Error> {
        let (newest, metadata) = match pool {
            ShieldedPool::Credit => {
                MostRecentShieldedAnchor::fetch_with_metadata(self, NoParamQuery {}, None).await?
            }
            ShieldedPool::Token(token_id) => {
                MostRecentShieldedAnchor::fetch_with_metadata(
                    self,
                    TokenShieldedPoolQuery { token_id },
                    None,
                )
                .await?
            }
        };
        Ok((
            newest.map(|MostRecentShieldedAnchor(anchor)| anchor),
            metadata,
        ))
    }

    /// Read every nonce a transition takes, each as its own proven read: the address nonces
    /// in one read, each identity and identity-contract nonce in one read of its own.
    async fn read_nonces(&self, nonces: &[NonceInput]) -> Result<Reads, Error> {
        let mut reads = Vec::with_capacity(nonces.len());

        let addresses: BTreeSet<PlatformAddress> = nonces
            .iter()
            .filter_map(|input| match input {
                NonceInput::Address { address, .. } => Some(*address),
                _ => None,
            })
            .collect();
        let address_infos = if addresses.is_empty() {
            None
        } else {
            let (infos, metadata) =
                AddressInfo::fetch_many_with_metadata(self, addresses, None).await?;
            Some((infos, metadata.height))
        };

        let mut identity_nonces: BTreeMap<Identifier, (Option<IdentityNonce>, u64)> =
            BTreeMap::new();
        let mut contract_nonces: BTreeMap<(Identifier, Identifier), (Option<IdentityNonce>, u64)> =
            BTreeMap::new();

        for input in nonces {
            let read = match *input {
                NonceInput::Address { address, nonce } => {
                    let Some((infos, height)) = address_infos.as_ref() else {
                        return Err(Error::Generic(
                            "address nonces were not read for an address input".to_string(),
                        ));
                    };
                    let info = infos.get(&address).ok_or_else(|| {
                        Error::InvalidProvedResponse(format!(
                            "the proven address infos leave out address {address:?}"
                        ))
                    })?;
                    InputRead {
                        // The network takes only the address's next nonce, and an address's
                        // nonce only grows: once it reaches this one, the input is taken. An
                        // address it does not know yet has taken none.
                        consumed: info.as_ref().is_some_and(|info| info.nonce >= nonce),
                        height: *height,
                    }
                }
                NonceInput::Identity { identity_id, nonce } => {
                    let (stored, height) = match identity_nonces.get(&identity_id) {
                        Some(read) => *read,
                        None => {
                            let (stored, metadata) =
                                IdentityNonceFetcher::fetch_with_metadata(self, identity_id, None)
                                    .await?;
                            let read = (stored.map(|stored| stored.0), metadata.height);
                            identity_nonces.insert(identity_id, read);
                            read
                        }
                    };
                    InputRead {
                        consumed: identity_nonce_taken(stored, nonce, identity_id),
                        height,
                    }
                }
                NonceInput::IdentityContract {
                    identity_id,
                    contract_id,
                    nonce,
                } => {
                    let key = (identity_id, contract_id);
                    let (stored, height) = match contract_nonces.get(&key) {
                        Some(read) => *read,
                        None => {
                            let (stored, metadata) =
                                IdentityContractNonceFetcher::fetch_with_metadata(self, key, None)
                                    .await?;
                            let read = (stored.map(|stored| stored.0), metadata.height);
                            contract_nonces.insert(key, read);
                            read
                        }
                    };
                    InputRead {
                        consumed: identity_nonce_taken(stored, nonce, identity_id),
                        height,
                    }
                }
            };
            reads.push(read);
        }

        Ok(Reads {
            verdict: judge_nonces(&reads),
            until_height: None,
            anchors_recorded_at: None,
        })
    }
}

/// What the reads of one transition showed.
struct Reads {
    verdict: InputsVerdict,
    /// The shielded spend's deadline, when it may still execute and one is proven.
    until_height: Option<u64>,
    /// A height by which every anchor of the shielded spend was recorded, when the reads found
    /// them all.
    anchors_recorded_at: Option<u64>,
}

/// Whether an identity or identity-contract nonce can no longer be used: the network refuses it
/// as already present, at the tip or in the past, or as too far in the past to fill. The
/// network's own merge rule decides, with the stored value the nonce validator reads. A nonce
/// that was never stored has taken nothing, and one too far in the future may still become
/// usable.
fn identity_nonce_taken(
    stored: Option<IdentityNonce>,
    nonce: IdentityNonce,
    identity_id: Identifier,
) -> bool {
    let Some(existing) = stored else {
        return false;
    };
    validate_identity_nonce_update(existing, nonce, identity_id)
        .errors
        .iter()
        .any(|error| match error {
            ConsensusError::StateError(StateError::InvalidIdentityNonceError(error)) => matches!(
                error.error(),
                MergeIdentityNonceResult::NonceAlreadyPresentAtTip
                    | MergeIdentityNonceResult::NonceAlreadyPresentInPast(_)
                    | MergeIdentityNonceResult::NonceTooFarInPast
            ),
            _ => false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_nonce::MISSING_IDENTITY_REVISIONS_MAX_BYTES;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;

    /// A shielded transfer with no actions, so no nullifier to read.
    fn shielded_transfer_without_actions() -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: Vec::new(),
                value_balance: 0,
                anchor: [0; 32],
                proof: Vec::new(),
                binding_signature: [0; 64],
            },
        ))
    }

    #[test]
    fn should_read_a_nonce_that_was_never_stored_as_free() {
        assert!(!identity_nonce_taken(None, 1, Identifier::default()));
        // Even one the network would refuse as too far ahead now: nothing has taken it.
        assert!(!identity_nonce_taken(None, 500, Identifier::default()));
    }

    #[test]
    fn should_read_the_stored_nonce_and_the_ones_below_it_as_taken() {
        assert!(identity_nonce_taken(Some(5), 5, Identifier::default()));
        assert!(identity_nonce_taken(Some(5), 4, Identifier::default()));
    }

    #[test]
    fn should_read_the_next_nonce_and_ones_slightly_ahead_as_free() {
        assert!(!identity_nonce_taken(Some(5), 6, Identifier::default()));
        assert!(!identity_nonce_taken(Some(5), 10, Identifier::default()));
    }

    #[test]
    fn should_read_a_nonce_too_far_ahead_as_free_since_it_may_become_usable() {
        let far = 5 + MISSING_IDENTITY_REVISIONS_MAX_BYTES + 1;
        assert!(!identity_nonce_taken(Some(5), far, Identifier::default()));
    }

    #[test]
    fn should_read_a_nonce_too_far_behind_as_taken_since_it_can_never_be_used() {
        let tip = 100;
        let behind = tip - MISSING_IDENTITY_REVISIONS_MAX_BYTES - 1;
        assert!(identity_nonce_taken(
            Some(tip),
            behind,
            Identifier::default()
        ));
    }

    #[test]
    fn should_read_a_skipped_nonce_still_marked_missing_as_free() {
        // Stored tip 10 with nonce 9 marked missing: bit (10 - 9 - 1) above the value bits.
        let stored = 10 | (1u64 << 40);
        assert!(!identity_nonce_taken(
            Some(stored),
            9,
            Identifier::default()
        ));
        // Nonce 8 is not marked missing, so it was used.
        assert!(identity_nonce_taken(Some(stored), 8, Identifier::default()));
    }

    #[test]
    fn should_refuse_a_transition_funded_by_an_asset_lock() {
        let transition = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
            IdentityTopUpTransitionV0::default(),
        ));
        let error = TransitionInputs::of(&transition).expect_err("asset lock types are refused");
        assert!(matches!(error, Error::InvalidRequest(_)), "{error:?}");
    }

    #[test]
    fn should_refuse_a_shielded_spend_without_nullifiers() {
        let transition = shielded_transfer_without_actions();
        let error = TransitionInputs::of(&transition).expect_err("nothing to read");
        assert!(matches!(error, Error::InvalidRequest(_)), "{error:?}");
    }
}

/// The reads through a mock SDK, whose answers all report height 0 and the latest protocol
/// version.
#[cfg(all(test, feature = "mocks"))]
mod sdk_tests {
    use super::*;
    use crate::SdkBuilder;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use drive_proof_verifier::types::ShieldedNullifierStatus;

    const ANCHOR: [u8; 32] = [0xA1; 32];
    const NEWER_ANCHOR: [u8; 32] = [0xB2; 32];
    const NULLIFIERS: [[u8; 32]; 2] = [[1; 32], [2; 32]];

    fn action(nullifier: [u8; 32]) -> SerializedAction {
        SerializedAction {
            nullifier,
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        }
    }

    fn shielded_transfer() -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions: NULLIFIERS.iter().copied().map(action).collect(),
                value_balance: 1_000,
                anchor: ANCHOR,
                proof: vec![8; 100],
                binding_signature: [9; 64],
            },
        ))
    }

    fn mock_sdk() -> Sdk {
        SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk")
    }

    async fn expect_anchors(sdk: &mut Sdk, anchors: Vec<[u8; 32]>) {
        sdk.mock()
            .expect_fetch(NoParamQuery {}, Some(ShieldedAnchors(anchors)))
            .await
            .expect("expected to register the anchor set");
    }

    async fn expect_nullifiers(sdk: &mut Sdk, spent: [bool; 2]) {
        let statuses = NULLIFIERS
            .iter()
            .zip(spent)
            .map(|(nullifier, is_spent)| ShieldedNullifierStatus {
                nullifier: *nullifier,
                is_spent,
            })
            .collect();
        sdk.mock()
            .expect_fetch(
                ShieldedNullifiersQuery(NULLIFIERS.to_vec()),
                Some(ShieldedNullifierStatuses(statuses)),
            )
            .await
            .expect("expected to register the nullifier statuses");
    }

    async fn expect_newest_anchor(sdk: &mut Sdk, anchor: [u8; 32]) {
        sdk.mock()
            .expect_fetch(NoParamQuery {}, Some(MostRecentShieldedAnchor(anchor)))
            .await
            .expect("expected to register the newest anchor");
    }

    #[tokio::test]
    async fn should_not_attribute_a_shielded_spend_whose_nullifiers_are_all_spent() {
        // Other transitions can spend the same notes, together or one by one.
        let mut sdk = mock_sdk();
        expect_anchors(&mut sdk, vec![]).await;
        expect_nullifiers(&mut sdk, [true, true]).await;

        let outcome = sdk
            .state_transition_outcome(&shielded_transfer(), OutcomeEvidence::default())
            .await
            .expect("expected an outcome");
        assert_eq!(outcome, TransitionOutcome::ExecutedOrSuperseded);
        assert!(outcome.is_final());
        assert!(!outcome.proves_not_executed());
    }

    #[tokio::test]
    async fn should_report_a_shielded_spend_whose_recorded_anchor_is_gone_as_expired() {
        let mut sdk = mock_sdk();
        expect_anchors(&mut sdk, vec![NEWER_ANCHOR]).await;
        expect_nullifiers(&mut sdk, [false, false]).await;

        // The wallet took the anchor from its tree synced to height 0, which the mock's reads
        // are at too.
        let outcome = sdk
            .state_transition_outcome(
                &shielded_transfer(),
                OutcomeEvidence::anchors_recorded_by(0),
            )
            .await
            .expect("expected an outcome");
        assert_eq!(
            outcome,
            TransitionOutcome::CannotExecute(CannotExecuteReason::AnchorExpired)
        );
        assert!(outcome.proves_not_executed());
    }

    #[tokio::test]
    async fn should_not_report_a_missing_anchor_expired_without_evidence_it_was_recorded() {
        // The answering node may not have reached the block that records the anchor yet.
        let mut sdk = mock_sdk();
        expect_anchors(&mut sdk, vec![NEWER_ANCHOR]).await;
        expect_nullifiers(&mut sdk, [false, false]).await;

        let outcome = sdk
            .state_transition_outcome(&shielded_transfer(), OutcomeEvidence::default())
            .await
            .expect("expected an outcome");
        assert_eq!(
            outcome,
            TransitionOutcome::StillPossible {
                checked_at_height: 0,
                anchors_recorded_at: None,
                until_height: None,
            }
        );
        assert!(!outcome.proves_not_executed());
    }

    #[tokio::test]
    async fn should_not_report_a_missing_anchor_expired_from_a_node_behind_the_evidence() {
        // The wallet saw its anchor recorded by height 5; this node answers from height 0.
        let mut sdk = mock_sdk();
        expect_anchors(&mut sdk, vec![NEWER_ANCHOR]).await;
        expect_nullifiers(&mut sdk, [false, false]).await;

        let outcome = sdk
            .state_transition_outcome(
                &shielded_transfer(),
                OutcomeEvidence::anchors_recorded_by(5),
            )
            .await
            .expect("expected an outcome");
        assert!(!outcome.proves_not_executed(), "{outcome:?}");
    }

    #[tokio::test]
    async fn should_report_a_shielded_spend_with_one_nullifier_taken_as_input_used() {
        let mut sdk = mock_sdk();
        expect_anchors(&mut sdk, vec![ANCHOR]).await;
        expect_nullifiers(&mut sdk, [true, false]).await;

        let outcome = sdk
            .state_transition_outcome(&shielded_transfer(), OutcomeEvidence::default())
            .await
            .expect("expected an outcome");
        assert_eq!(
            outcome,
            TransitionOutcome::CannotExecute(CannotExecuteReason::InputUsed)
        );
    }

    #[tokio::test]
    async fn should_give_a_pending_shielded_spend_its_anchor_deadline() {
        let mut sdk = mock_sdk();
        expect_anchors(&mut sdk, vec![ANCHOR, NEWER_ANCHOR]).await;
        expect_nullifiers(&mut sdk, [false, false]).await;
        expect_newest_anchor(&mut sdk, NEWER_ANCHOR).await;

        let outcome = sdk
            .state_transition_outcome(&shielded_transfer(), OutcomeEvidence::default())
            .await
            .expect("expected an outcome");
        // Read at height 0 with 1000 blocks kept and pruning every 100: the prune at the end of
        // block 1100 is the last one the anchor can survive.
        let constants = &PlatformVersion::latest()
            .drive_abci
            .validation_and_processing
            .event_constants;
        let expected = anchor_deadline(
            0,
            true,
            constants.shielded_anchor_retention_blocks,
            constants.shielded_anchor_pruning_interval,
        );
        assert_eq!(
            outcome,
            TransitionOutcome::StillPossible {
                checked_at_height: 0,
                anchors_recorded_at: Some(0),
                until_height: expected,
            }
        );
        assert!(!outcome.is_final());
    }

    #[tokio::test]
    async fn should_give_no_deadline_while_the_spend_s_anchor_is_the_newest() {
        let mut sdk = mock_sdk();
        expect_anchors(&mut sdk, vec![ANCHOR]).await;
        expect_nullifiers(&mut sdk, [false, false]).await;
        expect_newest_anchor(&mut sdk, ANCHOR).await;

        let outcome = sdk
            .state_transition_outcome(&shielded_transfer(), OutcomeEvidence::default())
            .await
            .expect("expected an outcome");
        assert_eq!(
            outcome,
            TransitionOutcome::StillPossible {
                checked_at_height: 0,
                anchors_recorded_at: Some(0),
                until_height: None,
            }
        );
    }

    fn credit_transfer(nonce: IdentityNonce) -> StateTransition {
        StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::new([7; 32]),
                recipient_id: Identifier::new([8; 32]),
                amount: 10_000,
                nonce,
                ..IdentityCreditTransferTransitionV0::default()
            },
        ))
    }

    #[tokio::test]
    async fn should_not_attribute_a_used_identity_nonce() {
        let mut sdk = mock_sdk();
        sdk.mock()
            .expect_fetch(Identifier::new([7; 32]), Some(IdentityNonceFetcher(5)))
            .await
            .expect("expected to register the nonce");

        let outcome = sdk
            .state_transition_outcome(&credit_transfer(5), OutcomeEvidence::default())
            .await
            .expect("expected an outcome");
        assert_eq!(outcome, TransitionOutcome::ExecutedOrSuperseded);
        assert!(outcome.is_final());
        assert!(!outcome.proves_not_executed());
    }

    #[tokio::test]
    async fn should_report_a_transition_whose_nonce_is_free_as_still_possible_without_deadline() {
        let mut sdk = mock_sdk();
        sdk.mock()
            .expect_fetch(Identifier::new([7; 32]), Some(IdentityNonceFetcher(4)))
            .await
            .expect("expected to register the nonce");

        let outcome = sdk
            .state_transition_outcome(&credit_transfer(5), OutcomeEvidence::default())
            .await
            .expect("expected an outcome");
        assert_eq!(
            outcome,
            TransitionOutcome::StillPossible {
                checked_at_height: 0,
                anchors_recorded_at: None,
                until_height: None,
            }
        );
    }
}
