//! Wallet-owned projection of replay verdicts onto durable input claims.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::{OutPoint, Txid};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;

use super::history_replay::{HistoryReplay, HistoryReplayError};
use crate::changeset::{
    swept_payment_status, AssetLockChangeSet, ClientWalletStartState, StoredTransaction,
};
use crate::wallet::asset_lock::tracked::AssetLockStatus;
use crate::wallet::identity::PaymentEntry;

/// One durable hold, independent of a backend's physical row representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredReplayClaim {
    /// Input held by persistence, including a placeholder without a funding output.
    pub outpoint: OutPoint,
    /// None preserves an anonymous hold; it never means permission to release.
    pub claimant: Option<Txid>,
    /// Existing lifetime stamp, retained when a new unmined winner has none.
    pub mined_height: Option<u32>,
}

/// The exact state a backend applies to one input after deleting rejected records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayClaimEffect {
    /// The funding itself was rejected: remove the output or placeholder.
    Remove,
    /// Release a materialized output, or delete a placeholder.
    Release,
    /// Keep or create a hold; a missing claimant preserves anonymous provenance.
    Hold {
        /// The surviving claimant, or the preexisting anonymous hold.
        claimant: Option<Txid>,
        /// The retained winner's own mining height, when available.
        mined_height: Option<u32>,
    },
}

/// Storage-independent effects computed from one authoritative wallet snapshot.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct HistoryReplayEffects {
    /// Physical history keys to remove from this wallet.
    pub removed_records: BTreeSet<Txid>,
    /// Rejected transaction identities whose outputs, proofs and operations are removed.
    pub removed_evidence: BTreeSet<Txid>,
    /// Apply after row deletion, whose storage cascades may clear old claimant links.
    pub claims: BTreeMap<OutPoint, ReplayClaimEffect>,
}

/// Operation retirement selected by the wallet, applied in the replay transaction.
#[derive(Debug, Default, PartialEq)]
pub struct HistoryReplayOperationEffects {
    /// Exact nonterminal asset-lock removals.
    pub asset_locks: AssetLockChangeSet,
    /// Exact payment overlays, preserving unrelated payment history.
    pub payments: BTreeMap<Identifier, BTreeMap<String, PaymentEntry>>,
}

impl HistoryReplayEffects {
    /// Select operation changes from the same fresh wallet snapshot as these effects.
    pub fn operation_effects(
        &self,
        state: &ClientWalletStartState,
    ) -> HistoryReplayOperationEffects {
        self.select_operations(
            state
                .unused_asset_locks
                .values()
                .flat_map(BTreeMap::values)
                .map(|lock| (lock.out_point, &lock.status)),
            state
                .identity_manager
                .wallet_identities
                .get(&state.wallet.compute_wallet_id())
                .into_iter()
                .flat_map(BTreeMap::values)
                .chain(
                    state
                        .identity_manager
                        .out_of_wallet_identities
                        .values()
                        .filter(|identity| {
                            identity.wallet_id == Some(state.wallet.compute_wallet_id())
                        }),
                )
                .map(|identity| (identity.identity.id(), &identity.dashpay().payments)),
        )
    }

    fn select_operations<'a>(
        &self,
        locks: impl Iterator<Item = (OutPoint, &'a AssetLockStatus)>,
        payments: impl Iterator<Item = (Identifier, &'a BTreeMap<String, PaymentEntry>)>,
    ) -> HistoryReplayOperationEffects {
        let mut effects = HistoryReplayOperationEffects::default();
        for (outpoint, status) in locks {
            if self.removed_evidence.contains(&outpoint.txid) && !status.is_consumed() {
                effects.asset_locks.removed.insert(outpoint);
            }
        }
        let retired: BTreeSet<_> = self.removed_evidence.iter().map(Txid::to_string).collect();
        for (identity, rows) in payments {
            for (txid, payment) in rows.iter().filter(|(txid, _)| retired.contains(*txid)) {
                if let Some(status) = swept_payment_status(payment) {
                    effects.payments.entry(identity).or_default().insert(
                        txid.clone(),
                        PaymentEntry {
                            status,
                            ..payment.clone()
                        },
                    );
                }
            }
        }
        effects
    }
}

#[cfg(test)]
mod operation_tests {
    use super::*;
    use crate::wallet::identity::PaymentStatus;
    use dashcore::hashes::Hash;

    #[test]
    fn should_settle_wallet_linked_payment_without_a_registration_index() {
        use crate::changeset::IdentityManagerStartState;
        use crate::wallet::identity::ManagedIdentity;
        use dpp::identity::{v0::IdentityV0, Identity};
        let wallet = key_wallet::Wallet::from_seed_bytes(
            [6; 64],
            dashcore::Network::Testnet,
            key_wallet::wallet::initialization::WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let wallet_id = wallet.compute_wallet_id();
        let txid = Txid::from_byte_array([7; 32]);
        let mut identities = IdentityManagerStartState::default();
        for (marker, owner) in [(8, Some(wallet_id)), (9, None)] {
            let id = Identifier::from([marker; 32]);
            let mut identity = ManagedIdentity::new_out_of_wallet(Identity::V0(IdentityV0 {
                id,
                public_keys: BTreeMap::new(),
                balance: 0,
                revision: 0,
            }));
            identity.wallet_id = owner;
            identity
                .dashpay_payments_mut()
                .insert(txid.to_string(), PaymentEntry::new_sent(id, 42, None));
            identities.out_of_wallet_identities.insert(id, identity);
        }
        let state = ClientWalletStartState {
            wallet_info: key_wallet::wallet::ManagedWalletInfo::from_wallet(&wallet, 0),
            wallet,
            identity_manager: identities,
            unused_asset_locks: BTreeMap::new(),
            unconfirmed_outgoing_txs: Vec::new(),
            recorded_history: Default::default(),
            dashpay_backfill: Default::default(),
        };
        let effects = HistoryReplayEffects {
            removed_evidence: [txid].into(),
            ..Default::default()
        };
        let operations = effects.operation_effects(&state);
        assert_eq!(operations.payments.len(), 1);
        assert_eq!(
            operations.payments[&Identifier::from([8; 32])][&txid.to_string()].status,
            PaymentStatus::Failed
        );
    }

    #[test]
    fn should_retire_only_rejected_nonterminal_asset_lock_operations() {
        let rejected = Txid::from_byte_array([1; 32]);
        let effects = HistoryReplayEffects {
            removed_evidence: [rejected].into(),
            ..Default::default()
        };
        let locks = [
            (
                OutPoint {
                    txid: rejected,
                    vout: 0,
                },
                AssetLockStatus::Broadcast,
            ),
            (
                OutPoint {
                    txid: rejected,
                    vout: 1,
                },
                AssetLockStatus::Consumed,
            ),
            (
                OutPoint {
                    txid: rejected,
                    vout: 2,
                },
                AssetLockStatus::DrainCompleted,
            ),
            (
                OutPoint {
                    txid: Txid::from_byte_array([2; 32]),
                    vout: 0,
                },
                AssetLockStatus::Broadcast,
            ),
        ];
        let result = effects.select_operations(
            locks.iter().map(|(key, status)| (*key, status)),
            std::iter::empty(),
        );
        assert_eq!(result.asset_locks.removed, [locks[0].0].into());
    }

    #[test]
    fn should_project_existing_payment_retirement_without_changing_received_rows() {
        let owner = Identifier::from([3; 32]);
        let ids: Vec<_> = (1..=4)
            .map(|byte| Txid::from_byte_array([byte; 32]))
            .collect();
        let sent = PaymentEntry::new_sent(owner, 42, Some("memo".into()));
        let mut confirmed = sent.clone();
        confirmed.status = PaymentStatus::Confirmed;
        let rows = [
            (ids[0].to_string(), sent.clone()),
            (ids[1].to_string(), confirmed),
            (
                ids[2].to_string(),
                PaymentEntry::new_received(owner, 42, None),
            ),
            (ids[3].to_string(), sent.clone()),
        ]
        .into();
        let effects = HistoryReplayEffects {
            removed_evidence: ids[..3].iter().copied().collect(),
            ..Default::default()
        };
        let result = effects.select_operations(std::iter::empty(), [(owner, &rows)].into_iter());
        assert_eq!(
            result.payments,
            [(
                owner,
                [
                    (
                        ids[0].to_string(),
                        PaymentEntry {
                            status: PaymentStatus::Failed,
                            ..sent.clone()
                        }
                    ),
                    (
                        ids[1].to_string(),
                        PaymentEntry {
                            status: PaymentStatus::Failed,
                            ..sent
                        }
                    ),
                ]
                .into()
            )]
            .into()
        );
    }
}

impl HistoryReplay {
    /// Project already-verified replay verdicts from one wallet snapshot, without new finality evidence.
    ///
    /// # Errors
    /// Returns [`HistoryReplayError::UnsettledAfterReplay`] if a final held winner
    /// was removed and neither a surviving body nor durable claim can protect its input,
    /// or an input has no authorization rejection or sweep permitting its release.
    pub fn storage_effects(
        &self,
        records: &[StoredTransaction],
        claims: &[StoredReplayClaim],
    ) -> Result<HistoryReplayEffects, HistoryReplayError> {
        let records_by_key: BTreeMap<_, _> = records
            .iter()
            .map(|record| (record.txid, (record.transaction.txid(), record)))
            .collect();
        // Sweep verdicts name physical keys; authorization names canonical bodies.
        let mut removed_records: BTreeSet<_> = self.defeated.iter().copied().collect();
        removed_records.extend(
            self.sweeps
                .iter()
                .flat_map(|sweep| sweep.txids.iter().copied()),
        );
        removed_records.extend(records_by_key.iter().filter_map(|(key, (body, _))| {
            self.authorization.rejected.contains(body).then_some(*key)
        }));
        let surviving_bodies: BTreeSet<_> = records_by_key
            .iter()
            .filter(|(key, _)| !removed_records.contains(*key))
            .map(|(_, (body, _))| *body)
            .collect();
        let mut removed_evidence = self.authorization.rejected.clone();
        removed_evidence.extend(removed_records.iter().copied());
        removed_evidence.extend(
            records_by_key
                .iter()
                .filter_map(|(key, (body, _))| removed_records.contains(key).then_some(*body)),
        );
        removed_evidence.retain(|txid| !surviving_bodies.contains(txid));
        let mut authorization_evidence = self.authorization.rejected.clone();
        authorization_evidence.extend(records_by_key.iter().filter_map(|(key, (body, _))| {
            self.authorization.rejected.contains(body).then_some(*key)
        }));
        authorization_evidence.retain(|txid| !surviving_bodies.contains(txid));

        let mut affected = BTreeSet::new();
        let mut release_allowed = BTreeSet::new();
        let mut survivors: BTreeMap<OutPoint, BTreeMap<Txid, Option<u32>>> = BTreeMap::new();
        for (key, (body, record)) in &records_by_key {
            for input in &record.transaction.input {
                if removed_records.contains(key) {
                    affected.insert(input.previous_output);
                    if self.authorization.rejected.contains(body) {
                        release_allowed.insert(input.previous_output);
                    }
                } else {
                    let height = record.context.block_info().map(|block| block.height());
                    survivors
                        .entry(input.previous_output)
                        .or_default()
                        .entry(*body)
                        .and_modify(|stamp| *stamp = merge_same_claim_stamp(*stamp, height))
                        .or_insert(height);
                }
            }
        }
        for (outpoint, claimants) in &self.authorization.surviving_claimants {
            for claimant in claimants
                .iter()
                .filter(|id| !removed_evidence.contains(*id))
            {
                survivors
                    .entry(*outpoint)
                    .or_default()
                    .entry(*claimant)
                    .or_insert(None);
            }
        }
        for (&input, &claimant) in &self.withheld_asset_lock_inputs {
            if removed_evidence.contains(&claimant) {
                return Err(HistoryReplayError::UnsettledAfterReplay);
            }
            affected.insert(input);
            survivors
                .entry(input)
                .or_default()
                .entry(claimant)
                .or_insert(None);
        }
        let mut stored_claims: BTreeMap<OutPoint, Vec<StoredReplayClaim>> = BTreeMap::new();
        for claim in claims {
            if claim
                .claimant
                .is_some_and(|id| authorization_evidence.contains(&id))
            {
                release_allowed.insert(claim.outpoint);
            }
            stored_claims
                .entry(claim.outpoint)
                .or_default()
                .push(*claim);
            // Deleting a protected physical alias can still clear its claimant links.
            if removed_evidence.contains(&claim.outpoint.txid)
                || claim.claimant.is_some_and(|id| {
                    removed_evidence.contains(&id) || removed_records.contains(&id)
                })
            {
                affected.insert(claim.outpoint);
            }
        }
        let mut pending: BTreeMap<_, _> = affected
            .iter()
            .filter_map(|outpoint| {
                stored_claims
                    .get(outpoint)
                    .and_then(|claims| select_claim(claims, |_| true))
                    .map(|claim| (*outpoint, claim))
            })
            .collect();
        let mut sweep_held = BTreeSet::new();
        for sweep in &self.sweeps {
            let mut loser_identities: BTreeSet<_> = sweep.txids.iter().copied().collect();
            let mut inputs: BTreeSet<_> = sweep.released_outpoints.iter().copied().collect();
            for loser in &sweep.txids {
                if let Some((body, record)) = records_by_key.get(loser) {
                    loser_identities.insert(*body);
                    inputs.extend(
                        record
                            .transaction
                            .input
                            .iter()
                            .map(|input| input.previous_output),
                    );
                }
            }
            // A placeholder survives its record, including an intermediate sweep winner.
            inputs.extend(claims.iter().chain(pending.values()).filter_map(|claim| {
                claim
                    .claimant
                    .filter(|id| loser_identities.contains(id))
                    .map(|_| claim.outpoint)
            }));
            let released: BTreeSet<_> = sweep.released_outpoints.iter().copied().collect();
            release_allowed.extend(released.iter().copied());
            for outpoint in inputs {
                affected.insert(outpoint);
                if released.contains(&outpoint) {
                    pending.remove(&outpoint);
                    sweep_held.remove(&outpoint);
                } else {
                    let stamp = sweep
                        .winner_mined_height
                        .or_else(|| pending.get(&outpoint).and_then(|claim| claim.mined_height));
                    pending.insert(
                        outpoint,
                        StoredReplayClaim {
                            outpoint,
                            claimant: Some(sweep.superseded_by),
                            mined_height: stamp,
                        },
                    );
                    sweep_held.insert(outpoint);
                }
            }
        }
        let mut projected = BTreeMap::new();
        for outpoint in affected {
            let strong = stored_claims.get(&outpoint).and_then(|claims| {
                select_claim(claims, |claim| {
                    claim
                        .claimant
                        .is_none_or(|id| !removed_evidence.contains(&id))
                })
            });
            let survivor = survivors.get(&outpoint).and_then(BTreeMap::first_key_value);
            let provisional = pending.get(&outpoint);
            let effect = if removed_evidence.contains(&outpoint.txid) {
                ReplayClaimEffect::Remove
            } else if let Some(claim) = strong {
                claim.effect()
            } else if sweep_held.contains(&outpoint) {
                let resolved = provisional.and_then(|claim| {
                    let id = claim.claimant?;
                    let claimant = if !removed_evidence.contains(&id) {
                        Some(id)
                    } else {
                        records_by_key
                            .get(&id)
                            .map(|(body, _)| *body)
                            .filter(|body| surviving_bodies.contains(body))
                    }?;
                    Some(ReplayClaimEffect::Hold {
                        claimant: Some(claimant),
                        mined_height: claim.mined_height,
                    })
                });
                if let Some(effect) = resolved {
                    effect
                } else if let Some((&claimant, &height)) = survivor {
                    ReplayClaimEffect::Hold {
                        claimant: Some(claimant),
                        mined_height: provisional.and_then(|claim| claim.mined_height).or(height),
                    }
                } else {
                    return Err(HistoryReplayError::UnsettledAfterReplay);
                }
            } else if let Some((&claimant, &height)) = survivor {
                ReplayClaimEffect::Hold {
                    claimant: Some(claimant),
                    mined_height: provisional.and_then(|claim| claim.mined_height).or(height),
                }
            } else if release_allowed.contains(&outpoint) {
                ReplayClaimEffect::Release
            } else {
                return Err(HistoryReplayError::UnsettledAfterReplay);
            };
            projected.insert(outpoint, effect);
        }
        Ok(HistoryReplayEffects {
            removed_records,
            removed_evidence,
            claims: projected,
        })
    }
}

impl StoredReplayClaim {
    fn effect(self) -> ReplayClaimEffect {
        ReplayClaimEffect::Hold {
            claimant: self.claimant,
            mined_height: self.mined_height,
        }
    }
}

// An unbounded observation cannot become collectible because another row has a stamp.
fn merge_same_claim_stamp(left: Option<u32>, right: Option<u32>) -> Option<u32> {
    left.zip(right).map(|(left, right)| left.max(right))
}

fn select_claim(
    claims: &[StoredReplayClaim],
    retain: impl Fn(&StoredReplayClaim) -> bool,
) -> Option<StoredReplayClaim> {
    let mut selected: Option<StoredReplayClaim> = None;
    for claim in claims.iter().filter(|claim| retain(claim)) {
        match selected.as_mut() {
            // Option ordering preserves anonymous provenance before any named claim.
            Some(prior) if prior.claimant < claim.claimant => {}
            Some(prior) if prior.claimant == claim.claimant => {
                prior.mined_height = merge_same_claim_stamp(prior.mined_height, claim.mined_height);
            }
            _ => selected = Some(*claim),
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::changeset::SweepBatch;
    use dashcore::{hashes::Hash, Transaction, TxIn};
    use key_wallet::transaction_checking::TransactionContext;

    fn point(marker: u8) -> OutPoint {
        OutPoint::new(Txid::from_byte_array([marker; 32]), 0)
    }

    fn record(marker: u32, inputs: &[OutPoint]) -> StoredTransaction {
        let transaction = Transaction {
            version: 1,
            lock_time: marker,
            input: inputs
                .iter()
                .map(|previous_output| TxIn {
                    previous_output: *previous_output,
                    ..Default::default()
                })
                .collect(),
            output: vec![],
            special_transaction_payload: None,
        };
        StoredTransaction {
            txid: transaction.txid(),
            transaction,
            context: TransactionContext::Mempool,
            stored_net_amount: None,
            stored_direction: None,
            owned_inputs: vec![],
        }
    }

    fn claim(outpoint: OutPoint, claimant: Option<Txid>) -> StoredReplayClaim {
        StoredReplayClaim {
            outpoint,
            claimant,
            mined_height: None,
        }
    }

    #[test]
    fn should_release_only_rejected_claims_and_keep_anonymous_or_recordless_holds() {
        let inputs = [point(1), point(2), point(3)];
        let bad = record(1, &inputs);
        let mut outcome = HistoryReplay::default();
        outcome.authorization.rejected.insert(bad.txid);
        let claims = [
            claim(inputs[0], Some(bad.txid)),
            claim(inputs[1], None),
            claim(inputs[2], Some(point(9).txid)),
        ];
        let effects = outcome
            .storage_effects(std::slice::from_ref(&bad), &claims)
            .unwrap();
        assert_eq!(effects.removed_records, [bad.txid].into());
        assert_eq!(effects.claims[&inputs[0]], ReplayClaimEffect::Release);
        assert_eq!(
            effects.claims[&inputs[1]],
            ReplayClaimEffect::Hold {
                claimant: None,
                mined_height: None
            }
        );
        assert_eq!(
            effects.claims[&inputs[2]],
            ReplayClaimEffect::Hold {
                claimant: Some(point(9).txid),
                mined_height: None
            }
        );
    }

    #[test]
    fn should_reassign_rejected_claims_to_a_surviving_body() {
        let input = point(4);
        let bad = record(1, &[input]);
        let survivor = record(2, &[input]);
        let mut outcome = HistoryReplay::default();
        outcome.authorization.rejected.insert(bad.txid);
        let effects = outcome
            .storage_effects(
                &[bad.clone(), survivor.clone()],
                &[claim(input, Some(bad.txid))],
            )
            .unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(survivor.txid),
                mined_height: None
            }
        );
    }

    #[test]
    fn should_project_sweep_releases_and_hold_unfunded_inputs_without_losing_names() {
        let inputs = [point(5), point(6)];
        let loser = record(1, &inputs);
        let child = record(2, &[OutPoint::new(loser.txid, 0)]);
        let winner = record(3, &[inputs[0]]);
        let mut outcome = HistoryReplay::default();
        outcome.defeated.extend([loser.txid, child.txid]);
        outcome.sweeps.push(SweepBatch {
            txids: vec![loser.txid, child.txid],
            superseded_by: winner.txid,
            winner_mined_height: Some(42),
            released_outpoints: vec![inputs[1]],
        });
        let effects = outcome
            .storage_effects(&[loser.clone(), child], &[])
            .unwrap();
        assert_eq!(
            effects.claims[&inputs[0]],
            ReplayClaimEffect::Hold {
                claimant: Some(winner.txid),
                mined_height: Some(42)
            }
        );
        assert_eq!(effects.claims[&inputs[1]], ReplayClaimEffect::Release);
        assert_eq!(
            effects.claims[&OutPoint::new(loser.txid, 0)],
            ReplayClaimEffect::Remove
        );
    }

    #[test]
    fn should_preserve_evidence_when_a_rejected_physical_key_names_a_surviving_body() {
        let input = point(7);
        let mut survivor = record(2, &[input]);
        let surviving_body = survivor.txid;
        survivor.txid = point(8).txid;
        let mut bad = record(1, &[input]);
        let bad_body = bad.txid;
        bad.txid = surviving_body;
        let mut outcome = HistoryReplay::default();
        outcome.authorization.rejected.insert(bad_body);
        let effects = outcome
            .storage_effects(
                &[bad, survivor.clone()],
                &[claim(input, Some(surviving_body))],
            )
            .unwrap();
        assert!(effects.removed_records.contains(&surviving_body));
        assert!(!effects.removed_evidence.contains(&surviving_body));
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(surviving_body),
                mined_height: None
            }
        );
    }

    #[test]
    fn should_map_defeated_physical_keys_without_removing_surviving_canonical_evidence() {
        let input = point(10);
        let mut survivor = record(2, &[input]);
        let survivor_body = survivor.txid;
        survivor.txid = point(11).txid;
        let mut loser = record(1, &[input]);
        let loser_body = loser.txid;
        loser.txid = survivor_body;
        let mut outcome = HistoryReplay::default();
        outcome.defeated.insert(loser.txid);
        let effects = outcome.storage_effects(&[loser, survivor], &[]).unwrap();
        assert_eq!(effects.removed_records, [survivor_body].into());
        assert_eq!(effects.removed_evidence, [loser_body].into());
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(survivor_body),
                mined_height: None,
            }
        );
    }

    fn chained(release: bool) -> (HistoryReplay, StoredTransaction, Txid, OutPoint) {
        let input = point(12);
        let loser = record(1, &[input]);
        let middle = point(13).txid;
        let winner = point(14).txid;
        let mut outcome = HistoryReplay::default();
        outcome.defeated.extend([loser.txid, middle]);
        outcome.sweeps = vec![
            SweepBatch {
                txids: vec![loser.txid],
                superseded_by: middle,
                winner_mined_height: Some(42),
                released_outpoints: vec![],
            },
            SweepBatch {
                txids: vec![middle],
                superseded_by: winner,
                winner_mined_height: None,
                released_outpoints: if release { vec![input] } else { vec![] },
            },
        ];
        (outcome, loser, winner, input)
    }

    #[test]
    fn should_follow_chained_claims_without_an_intermediate_body_and_keep_the_mined_stamp() {
        let (outcome, loser, winner, input) = chained(false);
        let effects = outcome.storage_effects(&[loser], &[]).unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(winner),
                mined_height: Some(42),
            }
        );
        assert!(!effects.removed_evidence.contains(&winner));
    }

    #[test]
    fn should_apply_a_later_release_to_a_recordless_intermediate_claim() {
        let (outcome, loser, _, input) = chained(true);
        let effects = outcome.storage_effects(&[loser], &[]).unwrap();
        assert_eq!(effects.claims[&input], ReplayClaimEffect::Release);
    }

    #[test]
    fn should_keep_anonymous_provenance_over_named_claims_and_a_sweep_winner() {
        let (outcome, loser, _, input) = chained(false);
        let mut claims = [claim(input, Some(point(15).txid)), claim(input, None)];
        claims[0].mined_height = Some(99);
        let effects = outcome
            .storage_effects(std::slice::from_ref(&loser), &claims)
            .unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: None,
                mined_height: None,
            }
        );
        claims.reverse();
        assert_eq!(effects, outcome.storage_effects(&[loser], &claims).unwrap());
    }

    #[test]
    fn should_choose_survivors_deterministically_and_preserve_existing_valid_provenance() {
        let input = point(16);
        let bad = record(1, &[input]);
        let a = record(2, &[input]);
        let b = record(3, &[input]);
        let mut outcome = HistoryReplay::default();
        outcome.authorization.rejected.insert(bad.txid);
        let mut records = [bad.clone(), a.clone(), b.clone()];
        let effects = outcome
            .storage_effects(&records, &[claim(input, Some(bad.txid))])
            .unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(a.txid.min(b.txid)),
                mined_height: None,
            }
        );
        records.reverse();
        assert_eq!(
            effects,
            outcome
                .storage_effects(&records, &[claim(input, Some(bad.txid))])
                .unwrap()
        );
        let retained = a.txid.max(b.txid);
        let effects = outcome
            .storage_effects(&records, &[claim(input, Some(retained))])
            .unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(retained),
                mined_height: None,
            }
        );
    }

    #[test]
    fn should_not_borrow_a_stamp_from_another_surviving_claimant() {
        let (outcome, loser, _, input) = chained(false);
        let survivor = point(17).txid;
        let mut claims = [claim(input, Some(survivor)), claim(input, Some(loser.txid))];
        claims[1].mined_height = Some(70);
        let effects = outcome
            .storage_effects(std::slice::from_ref(&loser), &claims)
            .unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(survivor),
                mined_height: None,
            }
        );
        claims[1].claimant = Some(survivor);
        claims[0].mined_height = Some(50);
        assert_eq!(
            outcome
                .storage_effects(std::slice::from_ref(&loser), &claims)
                .unwrap()
                .claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(survivor),
                mined_height: Some(70),
            }
        );
        claims[0].mined_height = None;
        assert_eq!(
            outcome.storage_effects(&[loser], &claims).unwrap().claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(survivor),
                mined_height: None,
            }
        );
    }

    #[test]
    fn should_remove_co_removed_funding_even_with_an_anonymous_hold() {
        let funding = record(1, &[]);
        let outpoint = OutPoint::new(funding.txid, 0);
        let mut outcome = HistoryReplay::default();
        outcome.authorization.rejected.insert(funding.txid);
        let effects = outcome
            .storage_effects(&[funding], &[claim(outpoint, None)])
            .unwrap();
        assert_eq!(effects.claims[&outpoint], ReplayClaimEffect::Remove);
    }

    #[test]
    fn should_leave_retired_only_records_and_unaffected_claims_unchanged() {
        let input = point(18);
        let unsafe_record = record(1, &[input]);
        let mut outcome = HistoryReplay::default();
        outcome.retired.insert(unsafe_record.txid);
        let claims = [claim(input, Some(unsafe_record.txid))];
        assert_eq!(
            outcome.storage_effects(&[unsafe_record], &claims).unwrap(),
            HistoryReplayEffects::default()
        );
    }

    #[test]
    fn should_not_delete_an_unrelated_physical_row_for_a_missing_rejected_body() {
        let mut survivor = record(1, &[]);
        let rejected = point(19).txid;
        survivor.txid = rejected;
        let mut outcome = HistoryReplay::default();
        outcome.authorization.rejected.insert(rejected);
        let effects = outcome.storage_effects(&[survivor], &[]).unwrap();
        assert!(effects.removed_records.is_empty());
    }

    #[test]
    fn should_fail_without_partial_effects_when_a_final_sweep_winner_cannot_be_resolved() {
        let (mut outcome, loser, winner, _) = chained(false);
        outcome.authorization.rejected.insert(winner);
        assert!(matches!(
            outcome.storage_effects(&[loser], &[]),
            Err(HistoryReplayError::UnsettledAfterReplay)
        ));
    }

    #[test]
    fn should_resolve_a_sweep_winners_removed_physical_alias_to_its_surviving_body() {
        let input = point(20);
        let mut loser = record(1, &[input]);
        let mut winner = record(2, &[input]);
        winner.txid = loser.txid;
        loser.txid = point(21).txid;
        let winner_body = winner.transaction.txid();
        let mut outcome = HistoryReplay::default();
        outcome.defeated.insert(loser.txid);
        outcome.sweeps.push(SweepBatch {
            txids: vec![loser.txid],
            superseded_by: winner.txid,
            winner_mined_height: Some(44),
            released_outpoints: vec![],
        });
        let effects = outcome.storage_effects(&[loser, winner], &[]).unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(winner_body),
                mined_height: Some(44),
            }
        );
    }

    #[test]
    fn should_restore_a_protected_alias_claim_even_outside_the_removed_bodys_inputs() {
        let input = point(22);
        let mut survivor = record(2, &[input]);
        let survivor_body = survivor.txid;
        survivor.txid = point(23).txid;
        let mut bad = record(1, &[]);
        let bad_body = bad.txid;
        bad.txid = survivor_body;
        let mut outcome = HistoryReplay::default();
        outcome.authorization.rejected.insert(bad_body);
        let effects = outcome
            .storage_effects(&[bad, survivor], &[claim(input, Some(survivor_body))])
            .unwrap();
        assert_eq!(
            effects.claims[&input],
            ReplayClaimEffect::Hold {
                claimant: Some(survivor_body),
                mined_height: None,
            }
        );
    }

    #[test]
    fn should_refuse_to_release_a_defeated_claim_without_a_sweep_verdict() {
        let input = point(24);
        let loser = record(1, &[input]);
        let mut outcome = HistoryReplay::default();
        outcome.defeated.insert(loser.txid);
        for claims in [vec![], vec![claim(input, Some(loser.txid))]] {
            assert!(matches!(
                outcome.storage_effects(std::slice::from_ref(&loser), &claims),
                Err(HistoryReplayError::UnsettledAfterReplay)
            ));
        }
    }

    #[test]
    fn should_leave_a_settled_recordless_chain_unchanged_on_repeated_projection() {
        let (outcome, loser, winner, input) = chained(false);
        let first = outcome.storage_effects(&[loser], &[]).unwrap();
        let ReplayClaimEffect::Hold {
            claimant,
            mined_height,
        } = first.claims[&input]
        else {
            panic!("the settled input must stay held")
        };
        assert_eq!(claimant, Some(winner));
        let second = outcome
            .storage_effects(
                &[],
                &[StoredReplayClaim {
                    outpoint: input,
                    claimant,
                    mined_height,
                }],
            )
            .unwrap();
        assert_eq!(first.removed_records, second.removed_records);
        assert_eq!(first.removed_evidence, second.removed_evidence);
        assert!(
            second.claims.is_empty(),
            "the surviving hold requires no rewrite"
        );
    }
}
