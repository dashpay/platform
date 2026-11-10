//! Pure owner-authorization planning for a selected portion of stored history.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::{OutPoint, TxOut, Txid};
use key_wallet::transaction_checking::spend_authorization::{
    verify_input_authorization, SpendAuthorizationError,
};

use crate::changeset::StoredTransaction;

/// Rejected history and the claims retained by every surviving transaction.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct HistoryAuthorization {
    /// Candidate transactions with a demonstrably invalid owner authorization.
    pub invalid_owner: BTreeSet<Txid>,
    /// Invalid owners and their descendants within the candidate subgraph.
    pub rejected: BTreeSet<Txid>,
    /// All surviving claimants, including transactions outside the candidate set.
    pub surviving_claimants: BTreeMap<OutPoint, BTreeSet<Txid>>,
}

/// Plan authorization repair without interpreting stored finality or changing state.
///
/// The caller selects candidates after resolving finality, supplies wallet-owned
/// prevouts, and validates that every `owned_inputs` annotation belongs to this
/// wallet. An annotation is an ownership hint, never proof of authorization.
/// Canonical funding bodies take precedence over remembered prevouts and hints;
/// a missing output in a present funding body is an invalid claim.
///
/// All identifiers are computed from transaction bodies, independently of stored
/// row keys. Unsupported authorization returns an error so callers can preserve
/// existing holds instead of treating unavailable verification as rejection.
pub(super) fn plan_history_authorization(
    records: &[StoredTransaction],
    candidates: &BTreeSet<Txid>,
    owned_prevouts: &BTreeMap<OutPoint, TxOut>,
) -> Result<HistoryAuthorization, SpendAuthorizationError> {
    let records: Vec<_> = records
        .iter()
        .map(|record| (record.transaction.txid(), record))
        .collect();
    let funding: BTreeMap<_, _> = records
        .iter()
        .map(|(txid, record)| (*txid, &record.transaction))
        .collect();
    let mut result = HistoryAuthorization::default();
    let mut children: BTreeMap<Txid, BTreeSet<Txid>> = BTreeMap::new();

    for (txid, record) in &records {
        if !candidates.contains(txid) {
            continue;
        }
        for (index, input) in record.transaction.input.iter().enumerate() {
            let outpoint = input.previous_output;
            children.entry(outpoint.txid).or_default().insert(*txid);
            let annotation = record
                .owned_inputs
                .iter()
                .find(|detail| usize::try_from(detail.index) == Ok(index));
            let remembered = owned_prevouts.get(&outpoint);
            if remembered.is_none() && annotation.is_none() {
                continue;
            }
            let fallback;
            let prevout = if let Some(parent) = funding.get(&outpoint.txid) {
                let Some(output) = usize::try_from(outpoint.vout)
                    .ok()
                    .and_then(|vout| parent.output.get(vout))
                else {
                    result.invalid_owner.insert(*txid);
                    continue;
                };
                output
            } else if let Some(remembered) = remembered {
                remembered
            } else if let Some(annotation) = annotation {
                fallback = TxOut {
                    value: annotation.value,
                    script_pubkey: annotation.address.script_pubkey(),
                };
                &fallback
            } else {
                continue;
            };
            if !verify_input_authorization(&record.transaction, index, prevout)? {
                result.invalid_owner.insert(*txid);
            }
        }
    }

    result.rejected = result.invalid_owner.clone();
    let mut pending = result.invalid_owner.clone();
    while let Some(parent) = pending.pop_first() {
        if let Some(descendants) = children.get(&parent) {
            for child in descendants {
                if result.rejected.insert(*child) {
                    pending.insert(*child);
                }
            }
        }
    }
    for (txid, record) in records {
        if !result.rejected.contains(&txid) {
            for input in &record.transaction.input {
                result
                    .surviving_claimants
                    .entry(input.previous_output)
                    .or_default()
                    .insert(txid);
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::crypto::ecdsa::EcdsaSecretKey;
    use dashcore::hashes::hash160;
    use dashcore::script::{Builder, PushBytesBuf};
    use dashcore::sighash::SighashCache;
    use dashcore::{Address, Network, ScriptBuf, Transaction, TxIn};
    use key_wallet::managed_account::transaction_record::InputDetail;
    use key_wallet::transaction_checking::TransactionContext;

    fn coin(key: u8) -> TxOut {
        let key = EcdsaSecretKey::from_bytes(&[key; 32]).unwrap();
        TxOut {
            value: 100_000,
            script_pubkey: ScriptBuf::new_p2pkh(
                &hash160::Hash::hash(&key.public_key().to_compressed()).into(),
            ),
        }
    }

    fn spend(outpoint: OutPoint, key: u8) -> StoredTransaction {
        let prevout = coin(key);
        let secret = EcdsaSecretKey::from_bytes(&[key; 32]).unwrap();
        let mut transaction = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: outpoint,
                ..Default::default()
            }],
            output: vec![prevout.clone()],
            special_transaction_payload: None,
        };
        let hash = SighashCache::new(&transaction)
            .legacy_signature_hash(0, &prevout.script_pubkey, 1)
            .unwrap();
        let mut signature = secret
            .sign(&hash.to_byte_array())
            .with_der(|der| der.to_vec());
        signature.push(1);
        transaction.input[0].script_sig = Builder::new()
            .push_slice(PushBytesBuf::try_from(signature).unwrap())
            .push_slice(secret.public_key().to_compressed())
            .into_script();
        StoredTransaction {
            txid: transaction.txid(),
            transaction,
            context: TransactionContext::Mempool,
            stored_net_amount: None,
            stored_direction: None,
            owned_inputs: vec![],
        }
    }

    fn outpoint(tag: u8) -> OutPoint {
        OutPoint {
            txid: Txid::from_byte_array([tag; 32]),
            vout: 0,
        }
    }

    fn annotate(record: &mut StoredTransaction, key: u8) {
        record.owned_inputs.push(InputDetail {
            index: 0,
            value: coin(key).value,
            address: Address::from_script(&coin(key).script_pubkey, Network::Testnet).unwrap(),
        });
    }

    fn candidates(records: &[StoredTransaction]) -> BTreeSet<Txid> {
        records.iter().map(|r| r.transaction.txid()).collect()
    }

    #[test]
    fn should_preserve_signed_claimants_and_reject_forged_claims_across_accounts() {
        let valid = spend(outpoint(1), 7);
        let forged = spend(outpoint(2), 8);
        let records = [valid.clone(), forged.clone()];
        let result = plan_history_authorization(
            &records,
            &candidates(&records),
            &BTreeMap::from([(outpoint(1), coin(7)), (outpoint(2), coin(7))]),
        )
        .unwrap();
        assert_eq!(
            result.invalid_owner,
            BTreeSet::from([forged.transaction.txid()])
        );
        assert_eq!(result.rejected, result.invalid_owner);
        assert_eq!(
            result.surviving_claimants,
            BTreeMap::from([(outpoint(1), BTreeSet::from([valid.transaction.txid()]))])
        );
    }

    #[test]
    fn should_reject_candidate_descendants_in_reverse_order_using_body_ids() {
        let mut invalid = spend(outpoint(1), 8);
        invalid.txid = outpoint(99).txid;
        let child = spend(
            OutPoint {
                txid: invalid.transaction.txid(),
                vout: 0,
            },
            8,
        );
        let grandchild = spend(
            OutPoint {
                txid: child.transaction.txid(),
                vout: 0,
            },
            8,
        );
        let records = [grandchild, child, invalid.clone()];
        let result = plan_history_authorization(
            &records,
            &candidates(&records),
            &BTreeMap::from([(outpoint(1), coin(7))]),
        )
        .unwrap();
        assert_eq!(
            result.invalid_owner,
            BTreeSet::from([invalid.transaction.txid()])
        );
        assert_eq!(result.rejected, candidates(&records));
        assert!(result.surviving_claimants.is_empty());
    }

    #[test]
    fn should_keep_every_surviving_claimant_including_non_candidates() {
        let invalid = spend(outpoint(1), 8);
        let final_record = spend(outpoint(1), 9);
        let valid = spend(outpoint(1), 7);
        let records = [invalid.clone(), final_record.clone(), valid.clone()];
        let selected = BTreeSet::from([invalid.transaction.txid(), valid.transaction.txid()]);
        let result = plan_history_authorization(
            &records,
            &selected,
            &BTreeMap::from([(outpoint(1), coin(7))]),
        )
        .unwrap();
        assert_eq!(
            result.rejected,
            BTreeSet::from([invalid.transaction.txid()])
        );
        assert_eq!(
            result.surviving_claimants[&outpoint(1)],
            BTreeSet::from([final_record.transaction.txid(), valid.transaction.txid()])
        );
    }

    #[test]
    fn should_use_canonical_funding_before_corrupt_annotation_or_coin() {
        let parent = spend(outpoint(1), 7);
        let prevout = OutPoint {
            txid: parent.transaction.txid(),
            vout: 0,
        };
        let mut valid = spend(prevout, 7);
        annotate(&mut valid, 8);
        let mut forged = spend(prevout, 8);
        annotate(&mut forged, 8);
        let selected = BTreeSet::from([valid.transaction.txid(), forged.transaction.txid()]);
        let result = plan_history_authorization(
            &[forged.clone(), valid, parent],
            &selected,
            &BTreeMap::from([(prevout, coin(8))]),
        )
        .unwrap();
        assert_eq!(
            result.invalid_owner,
            BTreeSet::from([forged.transaction.txid()])
        );
    }

    #[test]
    fn should_reject_missing_canonical_output_without_annotation_fallback() {
        let parent = spend(outpoint(1), 7);
        let prevout = OutPoint {
            txid: parent.transaction.txid(),
            vout: 5,
        };
        let mut child = spend(prevout, 7);
        annotate(&mut child, 7);
        let selected = BTreeSet::from([child.transaction.txid()]);
        let result = plan_history_authorization(
            &[child, parent],
            &selected,
            &BTreeMap::from([(prevout, coin(7))]),
        )
        .unwrap();
        assert_eq!(result.invalid_owner, selected);
    }

    #[test]
    fn should_verify_annotation_only_owned_inputs_and_ignore_external_inputs() {
        let mut valid = spend(outpoint(1), 7);
        annotate(&mut valid, 7);
        let mut forged = spend(outpoint(2), 8);
        annotate(&mut forged, 7);
        let mut external = spend(outpoint(3), 8);
        external.transaction.input[0].script_sig = ScriptBuf::new();
        let records = [valid.clone(), forged.clone(), external.clone()];
        let result =
            plan_history_authorization(&records, &candidates(&records), &BTreeMap::new()).unwrap();
        assert_eq!(result.rejected, BTreeSet::from([forged.transaction.txid()]));
        assert_eq!(
            result.surviving_claimants[&outpoint(1)],
            BTreeSet::from([valid.transaction.txid()])
        );
        assert_eq!(
            result.surviving_claimants[&outpoint(3)],
            BTreeSet::from([external.transaction.txid()])
        );
    }

    #[test]
    fn should_stop_descendant_rejection_at_a_non_candidate() {
        let invalid = spend(outpoint(1), 8);
        let final_record = spend(
            OutPoint {
                txid: invalid.transaction.txid(),
                vout: 0,
            },
            8,
        );
        let child = spend(
            OutPoint {
                txid: final_record.transaction.txid(),
                vout: 0,
            },
            8,
        );
        let records = [child.clone(), final_record.clone(), invalid.clone()];
        let selected = BTreeSet::from([invalid.transaction.txid(), child.transaction.txid()]);
        let result = plan_history_authorization(
            &records,
            &selected,
            &BTreeMap::from([(outpoint(1), coin(7))]),
        )
        .unwrap();
        assert_eq!(
            result.rejected,
            BTreeSet::from([invalid.transaction.txid()])
        );
        assert!(result
            .surviving_claimants
            .values()
            .any(|ids| ids.contains(&child.transaction.txid())));
        let mut reversed = records.to_vec();
        reversed.reverse();
        assert_eq!(
            result,
            plan_history_authorization(
                &reversed,
                &selected,
                &BTreeMap::from([(outpoint(1), coin(7))])
            )
            .unwrap()
        );
    }

    #[test]
    fn should_return_unknown_authorization_as_error_without_a_repair_plan() {
        let record = spend(outpoint(1), 7);
        let selected = BTreeSet::from([record.transaction.txid()]);
        let unknown = TxOut {
            value: 100_000,
            script_pubkey: ScriptBuf::new(),
        };
        assert_eq!(
            plan_history_authorization(
                &[record],
                &selected,
                &BTreeMap::from([(outpoint(1), unknown)])
            ),
            Err(SpendAuthorizationError::UnsupportedScript)
        );
    }
}
