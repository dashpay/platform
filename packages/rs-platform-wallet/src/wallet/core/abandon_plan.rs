//! Planning the removal of an outgoing transaction the network proved dead.
//!
//! A transaction the network will never accept still mutated the wallet when
//! it was sent: its change was credited and its inputs marked spent. Once a
//! probe (see [`crate::broadcast_probe`]) proves it dead, the wallet drops it
//! and every transaction built on its change — key-wallet's
//! `abandon_transaction_with_spends` (rust-dashcore#961) — and the persisters
//! must drop the same rows, or the next load hands the phantom change back.
//!
//! The removal reaches the persisters as an ordinary [`SweepBatch`], the
//! channel they already apply for conflict sweeps (#4558/#4559/#4589): the
//! batch's `txids` rows and every UTXO they created are deleted, and its
//! `released_outpoints` are handed back as unspent. A sweep normally names the
//! transaction that beat the losers in `superseded_by`, and persisters hold
//! every input *not* released under that winner. An abandon has no winner, so
//! the plan releases **every** input the chain took from outside itself and
//! `superseded_by` names the root — a label no persister reads, because a
//! released input is never held.
//!
//! That is only true while no other transaction of this wallet still claims
//! one of those inputs. If one did, a persister would keep that input held and
//! attribute the hold to `superseded_by` — here the root, a transaction that no
//! longer exists — leaving a hold nothing could ever resolve. The plan
//! therefore refuses to abandon in that case. It is not a case the probe
//! should produce anyway: a claimant already in a block or IS-locked makes
//! key-wallet sweep the chain itself, with the real winner, before any probe
//! runs; a claimant still in the mempool makes Core answer
//! `txn-mempool-conflict`, which the probe reports as refused, not dead.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::{OutPoint, Txid};

use crate::changeset::changeset::SweepBatch;

/// What the planner needs to know about one recorded transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordView {
    pub txid: Txid,
    pub inputs: Vec<OutPoint>,
    /// In a block or InstantSend-locked. A settled transaction spent real
    /// coins; it is never abandoned and never followed.
    pub settled: bool,
}

/// Why a proven-dead root cannot be abandoned as planned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AbandonRefusal {
    /// The wallet holds no record of the root — nothing to remove, or it was
    /// already swept by a conflicting spend.
    UnknownRoot,
    /// The root is in a block or IS-locked: whatever the probe said, it is
    /// not dead.
    SettledRoot,
    /// A transaction outside the abandoned chain also spends one of the
    /// chain's inputs, so releasing that input would contradict a surviving
    /// claim.
    InputClaimedBySurvivor { outpoint: OutPoint, claimant: Txid },
    /// A settled transaction spends an output of the chain, so the chain is
    /// on chain itself and not dead.
    SettledDescendant { txid: Txid },
}

/// The rows an abandon removes and the coins it hands back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbandonPlan {
    pub root: Txid,
    /// The root and every unsettled transaction built on it, transitively.
    pub txids: BTreeSet<Txid>,
    /// Every input the chain took from outside itself.
    pub released_outpoints: BTreeSet<OutPoint>,
}

impl AbandonPlan {
    /// The persistence batch for this plan — see the module docs for why
    /// `superseded_by` is the root and nothing is held.
    pub(crate) fn sweep_batch(&self) -> SweepBatch {
        SweepBatch {
            txids: self.txids.iter().copied().collect(),
            superseded_by: self.root,
            winner_mined_height: None,
            released_outpoints: self.released_outpoints.iter().copied().collect(),
        }
    }
}

/// Plan the abandon of `root`. `records` may list one transaction more than
/// once (key-wallet records a transaction in every account it touches).
pub(crate) fn plan_abandon<'a>(
    records: impl IntoIterator<Item = &'a RecordView>,
    root: Txid,
) -> Result<AbandonPlan, AbandonRefusal> {
    // One entry per transaction. A transaction recorded in several accounts
    // is the same transaction; if any account holds it as settled, it is.
    let mut by_txid: BTreeMap<Txid, (&[OutPoint], bool)> = BTreeMap::new();
    for record in records {
        by_txid
            .entry(record.txid)
            .and_modify(|(_, settled)| *settled |= record.settled)
            .or_insert((record.inputs.as_slice(), record.settled));
    }

    match by_txid.get(&root) {
        None => return Err(AbandonRefusal::UnknownRoot),
        Some((_, true)) => return Err(AbandonRefusal::SettledRoot),
        Some((_, false)) => {}
    }

    // The chain: the root plus every transaction spending an output of
    // something already in it, transitively. A settled spender ends the plan
    // instead of being skipped — see `SettledDescendant`.
    let mut txids = BTreeSet::from([root]);
    loop {
        let mut grew = false;
        for (txid, (inputs, settled)) in &by_txid {
            if txids.contains(txid) || !inputs.iter().any(|input| txids.contains(&input.txid)) {
                continue;
            }
            if *settled {
                return Err(AbandonRefusal::SettledDescendant { txid: *txid });
            }
            txids.insert(*txid);
            grew = true;
        }
        if !grew {
            break;
        }
    }

    let released_outpoints: BTreeSet<OutPoint> = txids
        .iter()
        .flat_map(|txid| by_txid[txid].0.iter().copied())
        .filter(|input| !txids.contains(&input.txid))
        .collect();

    // Nothing outside the chain may still claim a coin the chain releases:
    // the persister would hold it under `superseded_by`, and that is the
    // root, which is about to stop existing.
    for (claimant, (inputs, _)) in &by_txid {
        if txids.contains(claimant) {
            continue;
        }
        if let Some(outpoint) = inputs
            .iter()
            .find(|input| released_outpoints.contains(input))
        {
            return Err(AbandonRefusal::InputClaimedBySurvivor {
                outpoint: *outpoint,
                claimant: *claimant,
            });
        }
    }

    Ok(AbandonPlan {
        root,
        txids,
        released_outpoints,
    })
}

#[cfg(test)]
mod tests {
    use dashcore::hashes::Hash;

    use super::*;

    fn txid(n: u8) -> Txid {
        Txid::from_byte_array([n; 32])
    }

    fn outpoint(n: u8, vout: u32) -> OutPoint {
        OutPoint {
            txid: txid(n),
            vout,
        }
    }

    fn record(n: u8, inputs: &[OutPoint]) -> RecordView {
        RecordView {
            txid: txid(n),
            inputs: inputs.to_vec(),
            settled: false,
        }
    }

    fn settled(n: u8, inputs: &[OutPoint]) -> RecordView {
        RecordView {
            settled: true,
            ..record(n, inputs)
        }
    }

    #[test]
    fn should_release_every_input_of_a_lone_dead_transaction() {
        let records = [record(1, &[outpoint(90, 0), outpoint(91, 3)])];

        let plan = plan_abandon(&records, txid(1)).expect("abandonable");

        assert_eq!(plan.txids, BTreeSet::from([txid(1)]));
        assert_eq!(
            plan.released_outpoints,
            BTreeSet::from([outpoint(90, 0), outpoint(91, 3)])
        );
    }

    /// The customer's shape: sends built on the change of a dead send. The
    /// whole chain goes; the only coins handed back are the ones the chain
    /// took from outside itself — the change it spent internally was never
    /// real.
    #[test]
    fn should_take_the_whole_chain_and_release_only_its_outside_inputs() {
        let records = [
            record(1, &[outpoint(90, 0)]),
            record(2, &[outpoint(1, 1)]),
            record(3, &[outpoint(2, 1), outpoint(91, 0)]),
        ];

        let plan = plan_abandon(&records, txid(1)).expect("abandonable");

        assert_eq!(plan.txids, BTreeSet::from([txid(1), txid(2), txid(3)]));
        assert_eq!(
            plan.released_outpoints,
            BTreeSet::from([outpoint(90, 0), outpoint(91, 0)])
        );
    }

    /// The case that would leave a hold attributed to a transaction that no
    /// longer exists: another transaction of this wallet spends one of the
    /// chain's inputs. The plan must refuse rather than release it.
    #[test]
    fn should_refuse_when_a_surviving_transaction_claims_one_of_the_inputs() {
        let records = [
            record(1, &[outpoint(90, 0), outpoint(91, 0)]),
            record(7, &[outpoint(91, 0)]),
        ];

        assert_eq!(
            plan_abandon(&records, txid(1)),
            Err(AbandonRefusal::InputClaimedBySurvivor {
                outpoint: outpoint(91, 0),
                claimant: txid(7),
            })
        );
    }

    #[test]
    fn should_refuse_when_a_settled_transaction_claims_one_of_the_inputs() {
        let records = [
            record(1, &[outpoint(90, 0)]),
            settled(7, &[outpoint(90, 0)]),
        ];

        assert!(matches!(
            plan_abandon(&records, txid(1)),
            Err(AbandonRefusal::InputClaimedBySurvivor { .. })
        ));
    }

    /// A claimant deeper in the chain counts too: descendant 2 takes an
    /// outside coin that transaction 7 also spends.
    #[test]
    fn should_refuse_when_a_survivor_claims_an_input_of_a_descendant() {
        let records = [
            record(1, &[outpoint(90, 0)]),
            record(2, &[outpoint(1, 1), outpoint(92, 0)]),
            record(7, &[outpoint(92, 0)]),
        ];

        assert!(matches!(
            plan_abandon(&records, txid(1)),
            Err(AbandonRefusal::InputClaimedBySurvivor { claimant, .. }) if claimant == txid(7)
        ));
    }

    #[test]
    fn should_refuse_a_settled_root() {
        let records = [settled(1, &[outpoint(90, 0)])];

        assert_eq!(
            plan_abandon(&records, txid(1)),
            Err(AbandonRefusal::SettledRoot)
        );
    }

    #[test]
    fn should_refuse_a_root_the_wallet_does_not_hold() {
        let records = [record(2, &[outpoint(90, 0)])];

        assert_eq!(
            plan_abandon(&records, txid(1)),
            Err(AbandonRefusal::UnknownRoot)
        );
    }

    /// A settled transaction spending the chain's own output means the root
    /// is in a block too — a child cannot be mined without its parent — so
    /// whatever the probe said, the chain is not dead. key-wallet's walk
    /// would simply stop at the settled child; the planner refuses instead,
    /// because deleting the root's rows under a settled spender is exactly the
    /// state a persister cannot reason about.
    #[test]
    fn should_refuse_when_a_settled_transaction_spends_the_chain() {
        let records = [record(1, &[outpoint(90, 0)]), settled(2, &[outpoint(1, 1)])];

        assert_eq!(
            plan_abandon(&records, txid(1)),
            Err(AbandonRefusal::SettledDescendant { txid: txid(2) })
        );
    }

    /// key-wallet records a transaction once per account it touches; the
    /// duplicate must neither appear twice nor look like a rival claimant.
    #[test]
    fn should_treat_a_transaction_recorded_in_two_accounts_as_one() {
        let records = [
            record(1, &[outpoint(90, 0)]),
            record(1, &[outpoint(90, 0)]),
            record(2, &[outpoint(1, 1)]),
            record(2, &[outpoint(1, 1)]),
        ];

        let plan = plan_abandon(&records, txid(1)).expect("abandonable");

        assert_eq!(plan.txids, BTreeSet::from([txid(1), txid(2)]));
        assert_eq!(plan.released_outpoints, BTreeSet::from([outpoint(90, 0)]));
    }

    /// The batch every persister applies: nothing held, so nothing is ever
    /// attributed to `superseded_by`; no winner height, so no placeholder is
    /// ever stamped.
    #[test]
    fn should_build_a_batch_that_holds_nothing() {
        let records = [
            record(1, &[outpoint(90, 0)]),
            record(2, &[outpoint(1, 1), outpoint(91, 0)]),
        ];
        let plan = plan_abandon(&records, txid(1)).expect("abandonable");

        let batch = plan.sweep_batch();

        assert_eq!(batch.superseded_by, txid(1));
        assert_eq!(batch.winner_mined_height, None);
        assert_eq!(
            batch.txids.iter().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from([txid(1), txid(2)])
        );
        // Every input any removed transaction takes from outside the batch is
        // released — the property the persisters' "held" path depends on.
        let removed: BTreeSet<Txid> = batch.txids.iter().copied().collect();
        let released: BTreeSet<OutPoint> = batch.released_outpoints.iter().copied().collect();
        for record in &records {
            for input in &record.inputs {
                if !removed.contains(&input.txid) {
                    assert!(released.contains(input), "{input:?} would be held");
                }
            }
        }
    }
}
