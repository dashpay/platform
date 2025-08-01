use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

    /// Test if a transaction matches this filter
    pub fn matches_transaction(&self, tx_data: &[u8]) -> bool {
        // TODO: Implement proper transaction parsing and testing
        // This should extract outputs, inputs, and other relevant data
        // and test each against the bloom filter

        // For now, test the raw transaction data
        self.contains(tx_data)
    }

    /// Hash data using the specified hash function index
    fn hash_data(&self, data: &[u8], hash_func_index: u32) -> u32 {
        let mut hasher = DefaultHasher::new();

        // Include the hash function index and tweak in the hash
        hash_func_index.hash(&mut hasher);
        self.tweak.hash(&mut hasher);
        data.hash(&mut hasher);

        hasher.finish() as u32
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

/// Extract elements from a transaction for bloom filter testing

pub fn extract_transaction_elements(tx_data: &[u8]) -> Vec<Vec<u8>> {
    // TODO: Implement proper transaction parsing
    // This should extract:
    // - Transaction hash
    // - Output scripts
    // - Input previous transaction hashes
    // - Public keys
    // - Addresses

    // For now, return the transaction data itself
    vec![tx_data.to_vec()]
}

/// Test multiple elements against a bloom filter
/// Test elements against a bloom filter
pub fn test_elements_against_filter(filter: &TransactionFilter, elements: &[Vec<u8>]) -> bool {
    elements.iter().any(|element| filter.contains(element))
}

#[cfg(test)]
mod tests {
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
    fn test_element_extraction() {
        let tx_data = b"dummy_transaction_data";
        let elements = extract_transaction_elements(tx_data);

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0], tx_data.to_vec());
    }
}
