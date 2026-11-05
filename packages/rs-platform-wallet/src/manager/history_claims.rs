//! Wallet-owned projection of replay verdicts onto durable input claims.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::{OutPoint, Txid};

use super::history_replay::HistoryReplay;
use crate::changeset::StoredTransaction;

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

impl HistoryReplay {
    /// Project already-verified replay verdicts; this does not accept new finality evidence.
    pub fn storage_effects(
        &self,
        _records: &[StoredTransaction],
        _claims: &[StoredReplayClaim],
    ) -> HistoryReplayEffects {
        HistoryReplayEffects::default()
    }
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
        let effects = outcome.storage_effects(std::slice::from_ref(&bad), &claims);
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
        let effects = outcome.storage_effects(
            &[bad.clone(), survivor.clone()],
            &[claim(input, Some(bad.txid))],
        );
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
        let effects = outcome.storage_effects(&[loser.clone(), child], &[]);
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
        let effects = outcome.storage_effects(
            &[bad, survivor.clone()],
            &[claim(input, Some(surviving_body))],
        );
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
}
