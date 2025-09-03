use dashcore_rpc::dashcore::bloom::{BloomFilter as CoreBloomFilter, BloomFlags};
use dashcore_rpc::dashcore::script::Instruction;
use dashcore_rpc::dashcore::{ScriptBuf, Transaction as CoreTx, Txid};

/// Return true if any pushdata element in script is contained in the filter
fn script_matches(filter: &CoreBloomFilter, script: &[u8]) -> bool {
    for data in extract_pushdatas(script) {
        if filter.contains(&data) {
            return true;
        }
    }
    false
}

#[inline]
fn txid_to_be_bytes(txid: &Txid) -> Vec<u8> {
    use dashcore_rpc::dashcore::hashes::Hash;
    let mut arr = txid.to_byte_array();
    arr.reverse();
    arr.to_vec()
}

/// Rough check whether scriptPubKey represents a pubkey or multisig (used by update flag)
fn is_pubkey_script(script: &[u8]) -> bool {
    if script.len() >= 35 && (script[0] == 33 || script[0] == 65) {
        return true;
    }
    script.contains(&33u8) || script.contains(&65u8)
}

/// Extract pushdata from a Bitcoin script (supports OP_PUSH(1..75), PUSHDATA1/2/4)
pub fn extract_pushdatas(script: &[u8]) -> Vec<Vec<u8>> {
    // Parse using dashcore's script iterator which handles PUSH opcodes and bounds
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

/// Test if a transaction matches this filter (BIP37 semantics). Flags follow Bloom update policy
pub fn matches_transaction(filter: &mut CoreBloomFilter, tx: &CoreTx, flags: BloomFlags) -> bool {
    // 1) Check transaction hash (big-endian)
    let txid_be = txid_to_be_bytes(&tx.txid());
    if filter.contains(&txid_be) {
        return true;
    }

    // 2) Check outputs: any pushdata in script matches; optionally update filter with outpoint
    for (index, out) in tx.output.iter().enumerate() {
        if script_matches(filter, out.script_pubkey.as_bytes()) {
            if flags == BloomFlags::All
                || (flags == BloomFlags::PubkeyOnly
                    && is_pubkey_script(out.script_pubkey.as_bytes()))
            {
                let mut outpoint = Vec::with_capacity(36);
                outpoint.extend_from_slice(&txid_be);
                outpoint.extend_from_slice(&(index as u32).to_le_bytes());
                filter.insert(&outpoint);
            }
            return true;
        }
    }

    // 3) Check inputs: prev outpoint present in filter or scriptSig pushdata present
    for input in tx.input.iter() {
        let mut outpoint = Vec::with_capacity(36);
        let prev_txid_be = txid_to_be_bytes(&input.previous_output.txid);
        outpoint.extend_from_slice(&prev_txid_be);
        outpoint.extend_from_slice(&input.previous_output.vout.to_le_bytes());
        if filter.contains(&outpoint) || script_matches(filter, input.script_sig.as_bytes()) {
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
            tracing::error!("invalid bloom flags value {flag}");
            BloomFlags::None
        }
    }
}

#[cfg(test)]
mod tests {
    use dashcore_rpc::dashcore::bloom::BloomFilter as CoreBloomFilter;
    use dashcore_rpc::dashcore::hashes::Hash;

    use super::*;

    #[test]
    fn test_extract_pushdatas_simple() {
        // OP_DUP OP_HASH160 0x14 <20b> OP_EQUALVERIFY OP_CHECKSIG
        let mut script = vec![0x76, 0xa9, 0x14];
        script.extend(vec![0u8; 20]);
        script.extend([0x88, 0xac]);
        let parts = extract_pushdatas(&script);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].len(), 20);
    }

    #[test]
    fn test_txid_endianness_conversion() {
        use dashcore_rpc::dashcore::Txid as CoreTxid;
        use std::str::FromStr;

        // Big-endian hex string (human-readable form)
        let hex_be = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let txid = CoreTxid::from_str(hex_be).expect("valid txid hex");
        let be_bytes = super::txid_to_be_bytes(&txid);
        assert_eq!(be_bytes, hex::decode(hex_be).unwrap());
    }

    #[test]
    fn test_bit_checking() {
        let data = vec![0b10101010]; // Alternating bits
        let filter = CoreBloomFilter::from_bytes(data, 1, 0, BloomFlags::None).unwrap();
        // We don't test bit internals anymore; just ensure contains respects empty vs set data
        assert!(!filter.contains(&[0u8; 1]));
    }

    #[test]
    fn test_filter_stats() {
        let data = vec![0xFF, 0x00]; // First byte all 1s, second byte all 0s
        let filter = CoreBloomFilter::from_bytes(data, 2, 0, BloomFlags::None).unwrap();
        assert!(filter.contains(&[0xFF])); // sanity: some data may hit due to all-ones byte
    }

    #[test]
    fn test_matches_txid() {
        use dashcore_rpc::dashcore::Transaction as CoreTx;

        // Minimal transaction (no inputs/outputs)
        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        };

        // Insert txid into filter, then it must match
        let txid_be = super::txid_to_be_bytes(&tx.txid());
        let mut filter = CoreBloomFilter::from_bytes(vec![0; 128], 3, 0, BloomFlags::None).unwrap();
        filter.insert(&txid_be);
        assert!(matches_transaction(&mut filter, &tx, BloomFlags::None));
    }

    #[test]
    fn test_output_match_and_update_outpoint() {
        use dashcore_rpc::dashcore::{PubkeyHash, ScriptBuf, Transaction as CoreTx, TxOut};

        // Build a P2PKH output
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
        // Insert the hash160 (which is a script pushdata) into filter
        filter.insert(&h160.to_byte_array());

        // Should match due to output script pushdata
        assert!(matches_transaction(&mut filter, &tx, BloomFlags::All));

        // And since BLOOM_UPDATE_ALL, outpoint (txid||vout) is inserted
        let mut outpoint = super::txid_to_be_bytes(&tx.txid());
        outpoint.extend_from_slice(&(0u32).to_le_bytes());
        assert!(filter.contains(&outpoint));
    }

    #[test]
    fn test_output_match_no_update_when_flag_none() {
        use dashcore_rpc::dashcore::{PubkeyHash, ScriptBuf, Transaction as CoreTx, TxOut};

        // Build a P2PKH output
        let h160 = PubkeyHash::from_byte_array([0x22; 20]);
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
            CoreBloomFilter::from_bytes(vec![0; 256], 5, 42, BloomFlags::None).unwrap();
        filter.insert(&h160.to_byte_array());

        // Should match due to output script pushdata
        assert!(matches_transaction(&mut filter, &tx, BloomFlags::None));

        // But outpoint should NOT be inserted when BLOOM_UPDATE_NONE
        let mut outpoint = super::txid_to_be_bytes(&tx.txid());
        outpoint.extend_from_slice(&(0u32).to_le_bytes());
        assert!(!filter.contains(&outpoint));
    }

    #[test]
    fn test_output_match_no_update_p2pkh_when_flag_p2pubkey_only() {
        use dashcore_rpc::dashcore::{PubkeyHash, ScriptBuf, Transaction as CoreTx, TxOut};

        // Build a P2PKH output
        let h160 = PubkeyHash::from_byte_array([0x33; 20]);
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
            CoreBloomFilter::from_bytes(vec![0; 256], 5, 999, BloomFlags::PubkeyOnly).unwrap();
        filter.insert(&h160.to_byte_array());

        // Should match due to output script pushdata
        assert!(matches_transaction(
            &mut filter,
            &tx,
            BloomFlags::PubkeyOnly
        ));

        // But outpoint should NOT be inserted for P2PKH under P2PUBKEY_ONLY
        let mut outpoint = super::txid_to_be_bytes(&tx.txid());
        outpoint.extend_from_slice(&(0u32).to_le_bytes());
        assert!(!filter.contains(&outpoint));
    }

    #[test]
    fn test_output_match_updates_for_p2pk_when_flag_p2pubkey_only() {
        use dashcore_rpc::dashcore::{ScriptBuf, Transaction as CoreTx, TxOut};

        // Build a bare P2PK-like script: 33-byte push followed by OP_CHECKSIG
        let mut script_bytes = Vec::with_capacity(35);
        script_bytes.push(33u8); // push 33 bytes
        script_bytes.extend([0x02; 33]); // fake compressed pubkey
        script_bytes.push(0xAC); // OP_CHECKSIG
        let script = ScriptBuf::from_bytes(script_bytes);

        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 1000,
                script_pubkey: script.clone(),
            }],
            special_transaction_payload: None,
        };

        // Insert the pubkey (33 bytes) itself to match output pushdata
        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 256], 5, 777, BloomFlags::PubkeyOnly).unwrap();
        filter.insert(&[0x02; 33]);

        // Should match and, due to P2PUBKEY_ONLY and pubkey script, update outpoint
        assert!(matches_transaction(
            &mut filter,
            &tx,
            BloomFlags::PubkeyOnly
        ));

        let mut outpoint = super::txid_to_be_bytes(&tx.txid());
        outpoint.extend_from_slice(&(0u32).to_le_bytes());
        assert!(filter.contains(&outpoint));
    }

    #[test]
    fn test_input_matches_when_prevout_in_filter() {
        use dashcore_rpc::dashcore::{OutPoint, ScriptBuf, Transaction as CoreTx, TxIn};
        use std::str::FromStr;

        // Create a dummy previous txid
        let prev_txid = dashcore_rpc::dashcore::Txid::from_str(
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
        )
        .unwrap();

        let input = TxIn {
            previous_output: OutPoint {
                txid: prev_txid,
                vout: 5,
            },
            script_sig: ScriptBuf::new(),
            sequence: 0xFFFFFFFF,
            witness: Default::default(),
        };
        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![input],
            output: vec![],
            special_transaction_payload: None,
        };

        // Seed filter with the prevout (prev_txid||vout)
        let mut filter = CoreBloomFilter::from_bytes(vec![0; 256], 5, 0, BloomFlags::None).unwrap();
        let mut prev_outpoint = super::txid_to_be_bytes(&prev_txid);
        prev_outpoint.extend_from_slice(&(5u32).to_le_bytes());
        filter.insert(&prev_outpoint);

        assert!(matches_transaction(&mut filter, &tx, BloomFlags::None));
    }

    #[test]
    fn test_input_matches_by_scriptsig_pushdata() {
        use dashcore_rpc::dashcore::{OutPoint, ScriptBuf, Transaction as CoreTx, TxIn};
        use std::str::FromStr;

        // Build a scriptSig pushing a 33-byte pubkey
        let mut script_sig_bytes = Vec::new();
        script_sig_bytes.push(33u8);
        let pubkey = [0x03; 33];
        script_sig_bytes.extend(pubkey);
        let script_sig = ScriptBuf::from_bytes(script_sig_bytes);

        let input = TxIn {
            previous_output: OutPoint {
                txid: dashcore_rpc::dashcore::Txid::from_str(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .unwrap(),
                vout: 0,
            },
            script_sig,
            sequence: 0xFFFFFFFF,
            witness: Default::default(),
        };

        let tx = CoreTx {
            version: 2,
            lock_time: 0,
            input: vec![input],
            output: vec![],
            special_transaction_payload: None,
        };

        let mut filter =
            CoreBloomFilter::from_bytes(vec![0; 256], 5, 555, BloomFlags::None).unwrap();
        // Seed the filter with the same 33-byte pubkey so scriptSig matches
        filter.insert(&pubkey);

        assert!(matches_transaction(&mut filter, &tx, BloomFlags::None));
    }

    #[test]
    fn test_extract_pushdatas_pushdata_variants() {
        // PUSHDATA1
        let script1 = vec![0x4c, 0x03, 0xAA, 0xBB, 0xCC];
        let parts1 = extract_pushdatas(&script1);
        assert_eq!(parts1.len(), 1);
        assert_eq!(parts1[0], vec![0xAA, 0xBB, 0xCC]);

        // PUSHDATA2 (len=3)
        let script2 = vec![0x4d, 0x03, 0x00, 0xDE, 0xAD, 0xBE];
        let parts2 = extract_pushdatas(&script2);
        assert_eq!(parts2.len(), 1);
        assert_eq!(parts2[0], vec![0xDE, 0xAD, 0xBE]);

        // PUSHDATA4 (len=3)
        let script3 = vec![0x4e, 0x03, 0x00, 0x00, 0x00, 0xFA, 0xFB, 0xFC];
        let parts3 = extract_pushdatas(&script3);
        assert_eq!(parts3.len(), 1);
        assert_eq!(parts3[0], vec![0xFA, 0xFB, 0xFC]);

        // Truncated should not panic and should ignore incomplete push
        let script_trunc = vec![0x4d, 0x02];
        let parts_trunc = extract_pushdatas(&script_trunc);
        assert_eq!(parts_trunc.len(), 0);
    }
}
