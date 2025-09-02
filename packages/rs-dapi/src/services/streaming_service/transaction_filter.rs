use dashcore_rpc::dashcore::{Transaction as CoreTx, Txid};
use std::io::Cursor;

/// Bloom filter implementation for efficient transaction filtering

#[derive(Clone, Debug)]
pub struct TransactionFilter {
    /// Filter data (bit array)
    data: Vec<u8>,
    /// Number of hash functions
    hash_funcs: u32,
    /// Random tweak value
    tweak: u32,
    /// Update flags
    flags: u32,
}

impl TransactionFilter {
    /// Create a new transaction filter from bloom filter parameters
    pub fn new(data: Vec<u8>, hash_funcs: u32, tweak: u32, flags: u32) -> Self {
        Self {
            data,
            hash_funcs,
            tweak,
            flags,
        }
    }

    /// Test if the given data might be in the filter
    pub fn contains(&self, data: &[u8]) -> bool {
        if self.data.is_empty() || self.hash_funcs == 0 {
            return false;
        }

        let bit_count = self.data.len() * 8;

        for i in 0..self.hash_funcs {
            let hash = self.hash_data(data, i);
            let bit_index = (hash % bit_count as u32) as usize;

            if !self.is_bit_set(bit_index) {
                return false;
            }
        }

        true
    }

    /// Insert data into the filter (sets bits for each hash)
    pub fn insert(&mut self, data: &[u8]) {
        if self.data.is_empty() || self.hash_funcs == 0 {
            return;
        }
        let bit_count = self.data.len() * 8;
        for i in 0..self.hash_funcs {
            let hash = self.hash_data(data, i);
            let bit_index = (hash % bit_count as u32) as usize;
            self.set_bit(bit_index);
        }
    }

    /// Test if a transaction matches this filter (BIP37 semantics)
    pub fn matches_transaction(&mut self, tx: &CoreTx) -> bool {
        // 1) Check transaction hash (big-endian)
        let txid_be = txid_to_be_bytes(&tx.txid());
        if self.contains(&txid_be) {
            return true;
        }

        // 2) Check outputs: any pushdata in script matches; optionally update filter with outpoint
        for (index, out) in tx.output.iter().enumerate() {
            if script_matches(self, out.script_pubkey.as_bytes()) {
                // Update filter on match if flags allow
                const BLOOM_UPDATE_ALL: u32 = super::transaction_filter::BLOOM_UPDATE_ALL;
                const BLOOM_UPDATE_P2PUBKEY_ONLY: u32 =
                    super::transaction_filter::BLOOM_UPDATE_P2PUBKEY_ONLY;
                if self.flags == BLOOM_UPDATE_ALL
                    || (self.flags == BLOOM_UPDATE_P2PUBKEY_ONLY
                        && is_pubkey_script(out.script_pubkey.as_bytes()))
                {
                    let mut outpoint = Vec::with_capacity(36);
                    outpoint.extend_from_slice(&txid_be);
                    outpoint.extend_from_slice(&(index as u32).to_le_bytes());
                    self.insert(&outpoint);
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
            if self.contains(&outpoint) || script_matches(self, input.script_sig.as_bytes()) {
                return true;
            }
        }

        false
    }

    /// Hash data using the specified hash function index
    fn hash_data(&self, data: &[u8], hash_func_index: u32) -> u32 {
        // BIP37 Murmur3 32-bit with seed: (i * 0xFBA4C795 + nTweak)
        let seed = hash_func_index
            .wrapping_mul(0xFBA4C795)
            .wrapping_add(self.tweak);
        murmur3::murmur3_32(&mut Cursor::new(data), seed).unwrap_or(0)
    }

    /// Check if a bit is set in the filter
    fn is_bit_set(&self, bit_index: usize) -> bool {
        let byte_index = bit_index / 8;
        let bit_offset = bit_index % 8;

        if byte_index >= self.data.len() {
            return false;
        }

        (self.data[byte_index] >> bit_offset) & 1 == 1
    }

    /// Get filter statistics for debugging
    pub fn stats(&self) -> FilterStats {
        let total_bits = self.data.len() * 8;
        let set_bits = self
            .data
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum();

        FilterStats {
            total_bits,
            set_bits,
            hash_funcs: self.hash_funcs,
            data_size: self.data.len(),
            estimated_elements: self.estimate_element_count(),
            false_positive_rate: self.estimate_false_positive_rate(),
        }
    }

    /// Estimate the number of elements in the filter
    fn estimate_element_count(&self) -> f64 {
        if self.hash_funcs == 0 {
            return 0.0;
        }

        let m = (self.data.len() * 8) as f64; // Total bits
        let k = self.hash_funcs as f64; // Hash functions
        let x = self.count_set_bits() as f64; // Set bits

        if x >= m {
            return f64::INFINITY;
        }

        // Standard bloom filter element estimation formula
        -(m / k) * (1.0 - x / m).ln()
    }

    /// Estimate the false positive rate
    fn estimate_false_positive_rate(&self) -> f64 {
        if self.hash_funcs == 0 {
            return 0.0;
        }

        let m = (self.data.len() * 8) as f64;
        let k = self.hash_funcs as f64;
        let n = self.estimate_element_count();

        if n.is_infinite() || n <= 0.0 {
            return 1.0;
        }

        // Standard bloom filter false positive rate formula
        (1.0 - (-k * n / m).exp()).powf(k)
    }

    /// Count the number of set bits in the filter
    fn count_set_bits(&self) -> usize {
        self.data
            .iter()
            .map(|byte| byte.count_ones() as usize)
            .sum()
    }

    fn set_bit(&mut self, bit_index: usize) {
        let byte_index = bit_index / 8;
        let bit_offset = bit_index % 8;
        if byte_index < self.data.len() {
            self.data[byte_index] |= 1u8 << bit_offset;
        }
    }
}

/// Statistics about a bloom filter

#[derive(Debug, Clone)]
pub struct FilterStats {
    pub total_bits: usize,
    pub set_bits: usize,
    pub hash_funcs: u32,
    pub data_size: usize,
    pub estimated_elements: f64,
    pub false_positive_rate: f64,
}

/// Flags matching dashcore-lib for filter update behavior
pub const BLOOM_UPDATE_NONE: u32 = 0;
pub const BLOOM_UPDATE_ALL: u32 = 1;
pub const BLOOM_UPDATE_P2PUBKEY_ONLY: u32 = 2;

// We use dashcore::Transaction directly; no local ParsedTransaction necessary.

/// Return true if any pushdata element in script is contained in the filter
fn script_matches(filter: &TransactionFilter, script: &[u8]) -> bool {
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
    let mut i = 0usize;
    let mut parts = Vec::new();
    while i < script.len() {
        let op = script[i];
        i += 1;
        let len = if (1..=75).contains(&op) {
            op as usize
        } else if op == 0x4c {
            if i >= script.len() {
                break;
            }
            let l = script[i] as usize;
            i += 1;
            l
        } else if op == 0x4d {
            if i + 1 >= script.len() {
                break;
            }
            let l = u16::from_le_bytes([script[i], script[i + 1]]) as usize;
            i += 2;
            l
        } else if op == 0x4e {
            if i + 3 >= script.len() {
                break;
            }
            let l = u32::from_le_bytes([script[i], script[i + 1], script[i + 2], script[i + 3]])
                as usize;
            i += 4;
            l
        } else {
            continue;
        };
        if i + len > script.len() {
            break;
        }
        parts.push(script[i..i + len].to_vec());
        i += len;
    }
    parts
}

#[cfg(test)]
mod tests {
    use dashcore_rpc::dashcore::hashes::Hash;

    use super::*;

    #[test]
    fn test_empty_filter() {
        let filter = TransactionFilter::new(vec![], 0, 0, 0);
        assert!(!filter.contains(b"test"));
    }

    #[test]
    fn test_filter_creation() {
        let data = vec![0xFF, 0x00, 0xFF]; // Some bit pattern
        let filter = TransactionFilter::new(data.clone(), 3, 12345, 0);

        assert_eq!(filter.data, data);
        assert_eq!(filter.hash_funcs, 3);
        assert_eq!(filter.tweak, 12345);
    }

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
        let filter = TransactionFilter::new(data, 1, 0, 0);

        // Bit 0 should be 0, bit 1 should be 1, etc.
        assert!(!filter.is_bit_set(0));
        assert!(filter.is_bit_set(1));
        assert!(!filter.is_bit_set(2));
        assert!(filter.is_bit_set(3));
    }

    #[test]
    fn test_filter_stats() {
        let data = vec![0xFF, 0x00]; // First byte all 1s, second byte all 0s
        let filter = TransactionFilter::new(data, 2, 0, 0);

        let stats = filter.stats();
        assert_eq!(stats.total_bits, 16);
        assert_eq!(stats.set_bits, 8);
        assert_eq!(stats.hash_funcs, 2);
        assert_eq!(stats.data_size, 2);
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
        let mut filter = TransactionFilter::new(vec![0; 128], 3, 0, super::BLOOM_UPDATE_NONE);
        filter.insert(&txid_be);
        assert!(filter.matches_transaction(&tx));
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

        let mut filter = TransactionFilter::new(vec![0; 256], 5, 12345, super::BLOOM_UPDATE_ALL);
        // Insert the hash160 (which is a script pushdata) into filter
        filter.insert(&h160.to_byte_array());

        // Should match due to output script pushdata
        assert!(filter.matches_transaction(&tx));

        // And since BLOOM_UPDATE_ALL, outpoint (txid||vout) is inserted
        let mut outpoint = super::txid_to_be_bytes(&tx.txid());
        outpoint.extend_from_slice(&(0u32).to_le_bytes());
        assert!(filter.contains(&outpoint));
    }
}
