use std::sync::Arc;

use dash_spv::bloom::utils::{extract_pubkey_hash, outpoint_to_bytes};
use dashcore_rpc::dashcore::bloom::{BloomFilter as CoreBloomFilter, BloomFlags};
use dashcore_rpc::dashcore::script::Instruction;
use dashcore_rpc::dashcore::{OutPoint, ScriptBuf, Transaction as CoreTx, Txid};

fn script_matches(filter: &CoreBloomFilter, script: &ScriptBuf) -> bool {
    let script_bytes = script.as_bytes();
    if filter.contains(script_bytes) {
        return true;
    }

    if let Some(pubkey_hash) = extract_pubkey_hash(script.as_script())
        && filter.contains(&pubkey_hash) {
            return true;
        }

    extract_pushdatas(script_bytes)
        .into_iter()
        .any(|data| filter.contains(&data))
}

#[inline]
fn txid_to_be_bytes(txid: &Txid) -> Vec<u8> {
    use dashcore_rpc::dashcore::hashes::Hash;
    let mut arr = txid.to_byte_array();
    arr.reverse();
    arr.to_vec()
}

fn is_pubkey_script(script: &ScriptBuf) -> bool {
    let bytes = script.as_bytes();
    if bytes.len() >= 35 && (bytes[0] == 33 || bytes[0] == 65) {
        return true;
    }
    bytes.contains(&33u8)
        || bytes.contains(&65u8)
        || extract_pubkey_hash(script.as_script()).is_some()
}

pub fn extract_pushdatas(script: &[u8]) -> Vec<Vec<u8>> {
    let script_buf = ScriptBuf::from_bytes(script.to_vec());
    script_buf
        .as_script()
        .instructions()
        .filter_map(|res| match res {
            Ok(Instruction::PushBytes(pb)) => Some(pb.as_bytes().to_vec()),
            _ => None,
        })
        .collect()
}

pub fn matches_transaction(
    filter_lock: Arc<std::sync::RwLock<CoreBloomFilter>>,
    tx: &CoreTx,
    flags: BloomFlags,
) -> bool {
    let filter = match filter_lock.read().inspect_err(|e| {
        tracing::debug!("Failed to acquire read lock for bloom filter: {}", e);
    }) {
        Ok(guard) => guard,
        Err(_) => return false,
    };

    let txid = tx.txid();
    let txid_be = txid_to_be_bytes(&txid);
    if filter.contains(&txid_be) {
        return true;
    }

    for (index, out) in tx.output.iter().enumerate() {
        if script_matches(&filter, &out.script_pubkey) {
            if flags == BloomFlags::All
                || (flags == BloomFlags::PubkeyOnly && is_pubkey_script(&out.script_pubkey))
            {
                let outpoint_bytes = outpoint_to_bytes(&OutPoint {
                    txid,
                    vout: index as u32,
                });
                drop(filter);
                if let Ok(mut f) = filter_lock.write().inspect_err(|e| {
                    tracing::debug!("Failed to acquire write lock for bloom filter: {}", e);
                }) {
                    f.insert(&outpoint_bytes);
                }
            }
            return true;
        }
    }

    for input in tx.input.iter() {
        let outpoint_bytes = outpoint_to_bytes(&input.previous_output);
        if filter.contains(&outpoint_bytes) || script_matches(&filter, &input.script_sig) {
            return true;
        }
    }

    false
}

pub(crate) fn bloom_flags_from_int<I: TryInto<u8>>(flags: I) -> BloomFlags {
    let flag = flags.try_into().unwrap_or(u8::MAX);
    match flag {
        0 => BloomFlags::None,
        1 => BloomFlags::All,
        2 => BloomFlags::PubkeyOnly,
        _ => {
            tracing::debug!("invalid bloom flags value {flag}");
            BloomFlags::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_spv::bloom::utils::outpoint_to_bytes;
    use dashcore_rpc::dashcore::bloom::BloomFilter as CoreBloomFilter;
    use dashcore_rpc::dashcore::hashes::Hash;
    use dashcore_rpc::dashcore::{OutPoint, PubkeyHash};
    use std::str::FromStr;
    use std::sync::RwLock;

    #[test]
    fn test_insert_and_contains_roundtrip() {
        let mut filter = CoreBloomFilter::from_bytes(vec![0; 128], 3, 0, BloomFlags::None).unwrap();
        let key = b"hello";
        assert!(!filter.contains(key));
        filter.insert(key);
        assert!(filter.contains(key));
    }

    #[test]
    fn test_extract_pushdatas_simple() {
        let mut script = vec![0x76, 0xa9, 0x14];
        script.extend(vec![0u8; 20]);
        script.extend([0x88, 0xac]);
        let parts = extract_pushdatas(&script);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 20);
    }

    #[test]
    fn test_txid_endianness_conversion() {
        let hex_be = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let txid = Txid::from_str(hex_be).expect("valid txid hex");
        let be_bytes = super::txid_to_be_bytes(&txid);
        assert_eq!(be_bytes, hex::decode(hex_be).unwrap());
    }

    #[test]
    fn test_matches_txid() {
        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };
        let txid_be = super::txid_to_be_bytes(&tx.txid());
        let mut filter = CoreBloomFilter::from_bytes(vec![0; 128], 3, 0, BloomFlags::None).unwrap();
        filter.insert(&txid_be);
        assert!(matches_transaction(
            Arc::new(RwLock::new(filter)),
            &tx,
            BloomFlags::None
        ));
    }

    #[test]
    fn test_output_match_and_update_outpoint() {
        use dashcore_rpc::dashcore::{ScriptBuf, Transaction as CoreTx, TxOut};
        let h160 = PubkeyHash::from_byte_array([0x11; 20]);
        let script = ScriptBuf::new_p2pkh(&h160);
        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 1000,
                script_pubkey: script,
            }],
            special_transaction_payload: None,
        };
        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 256], 5, 12345, BloomFlags::All).unwrap();
        filter.insert(&h160.to_byte_array());
        let filter_lock = Arc::new(RwLock::new(filter));
        assert!(matches_transaction(
            filter_lock.clone(),
            &tx,
            BloomFlags::All
        ));
        let outpoint = outpoint_to_bytes(&OutPoint {
            txid: tx.txid(),
            vout: 0,
        });
        let guard = filter_lock.read().unwrap();
        assert!(guard.contains(&outpoint));
    }

    #[test]
    fn test_all_flag_updates_enable_second_tx_match() {
        use dashcore_rpc::dashcore::{ScriptBuf, Transaction as CoreTx, TxIn, TxOut};
        let h160 = PubkeyHash::from_byte_array([0x55; 20]);
        let script = ScriptBuf::new_p2pkh(&h160);
        let tx_a = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 1000,
                script_pubkey: script,
            }],
            special_transaction_payload: None,
        };
        let tx_b = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: tx_a.txid(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: 0xFFFFFFFF,
                witness: Default::default(),
            }],
            output: vec![],
            special_transaction_payload: None,
        };
        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 1024], 5, 123, BloomFlags::All).unwrap();
        filter.insert(&h160.to_byte_array());
        let filter_lock = Arc::new(RwLock::new(filter));
        assert!(matches_transaction(
            filter_lock.clone(),
            &tx_a,
            BloomFlags::All
        ));
        assert!(matches_transaction(
            filter_lock.clone(),
            &tx_b,
            BloomFlags::All
        ));
    }

    #[test]
    fn test_none_flag_does_not_update_for_second_tx() {
        use dashcore_rpc::dashcore::{ScriptBuf, Transaction as CoreTx, TxIn, TxOut};
        let h160 = PubkeyHash::from_byte_array([0x66; 20]);
        let script = ScriptBuf::new_p2pkh(&h160);
        let tx_a = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 1000,
                script_pubkey: script,
            }],
            special_transaction_payload: None,
        };
        let tx_b = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: tx_a.txid(),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: 0xFFFFFFFF,
                witness: Default::default(),
            }],
            output: vec![],
            special_transaction_payload: None,
        };
        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 2048], 5, 456, BloomFlags::None).unwrap();
        filter.insert(&h160.to_byte_array());
        let filter_lock = Arc::new(RwLock::new(filter));
        assert!(matches_transaction(
            filter_lock.clone(),
            &tx_a,
            BloomFlags::None
        ));
        assert!(!matches_transaction(
            filter_lock.clone(),
            &tx_b,
            BloomFlags::None
        ));
    }

    #[test]
    fn test_p2sh_and_opreturn_do_not_update_under_pubkeyonly() {
        use dashcore_rpc::dashcore::{ScriptBuf, ScriptHash, Transaction as CoreTx, TxOut};
        let sh = ScriptHash::from_byte_array([0x77; 20]);
        let p2sh = ScriptBuf::new_p2sh(&sh);
        let mut opret_bytes = Vec::new();
        opret_bytes.push(0x6a);
        opret_bytes.push(8u8);
        opret_bytes.extend([0xAB; 8]);
        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 1024], 5, 789, BloomFlags::PubkeyOnly).unwrap();
        filter.insert(&sh.to_byte_array());
        filter.insert(&[0xAB; 8]);
        let tx_sh = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 1,
                script_pubkey: p2sh,
            }],
            special_transaction_payload: None,
        };
        let filter_lock = Arc::new(RwLock::new(filter));
        assert!(matches_transaction(
            filter_lock.clone(),
            &tx_sh,
            BloomFlags::PubkeyOnly
        ));
        let outpoint = outpoint_to_bytes(&OutPoint {
            txid: tx_sh.txid(),
            vout: 0,
        });
        assert!(!filter_lock.read().unwrap().contains(&outpoint));
        let mut opret_bytes2 = Vec::new();
        opret_bytes2.push(0x6a);
        opret_bytes2.push(8u8);
        opret_bytes2.extend([0xAB; 8]);
        let tx_or = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 0,
                script_pubkey: ScriptBuf::from_bytes(opret_bytes2),
            }],
            special_transaction_payload: None,
        };
        assert!(matches_transaction(
            filter_lock.clone(),
            &tx_or,
            BloomFlags::PubkeyOnly
        ));
        let outpoint2 = outpoint_to_bytes(&OutPoint {
            txid: tx_or.txid(),
            vout: 0,
        });
        assert!(!filter_lock.read().unwrap().contains(&outpoint2));
    }

    #[test]
    fn test_nonminimal_push_still_matches() {
        use dashcore_rpc::dashcore::{ScriptBuf, Transaction as CoreTx, TxOut};
        let script = ScriptBuf::from_bytes(vec![0x4c, 0x03, 0xDE, 0xAD, 0xBE]);
        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 1,
                script_pubkey: script,
            }],
            special_transaction_payload: None,
        };
        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 1024], 5, 321, BloomFlags::None).unwrap();
        filter.insert(&[0xDE, 0xAD, 0xBE]);
        let filter_lock = Arc::new(RwLock::new(filter));
        assert!(matches_transaction(
            filter_lock.clone(),
            &tx,
            BloomFlags::None
        ));
    }

    #[test]
    fn test_witness_only_pushdata_does_not_match() {
        use dashcore_rpc::dashcore::{OutPoint, ScriptBuf, Transaction as CoreTx, TxIn, TxOut};
        let pubkey = [0x02; 33];
        let input = TxIn {
            previous_output: OutPoint {
                txid: Txid::from_str(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .unwrap(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: 0xFFFFFFFF,
            witness: vec![pubkey.to_vec()].into(),
        };
        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![input],
            output: vec![TxOut {
                value: 0,
                script_pubkey: ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        };
        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 4096], 5, 654, BloomFlags::None).unwrap();
        filter.insert(&pubkey);
        let filter_lock = Arc::new(RwLock::new(filter));
        assert!(!matches_transaction(
            filter_lock.clone(),
            &tx,
            BloomFlags::None
        ));
    }

    #[test]
    fn test_bloom_flags_from_int_mapping() {
        assert!(matches!(bloom_flags_from_int(0u32), BloomFlags::None));
        assert!(matches!(bloom_flags_from_int(1u32), BloomFlags::All));
        assert!(matches!(bloom_flags_from_int(2u32), BloomFlags::PubkeyOnly));
        assert!(matches!(bloom_flags_from_int(255u32), BloomFlags::None));
    }
}
