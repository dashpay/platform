/// Operations performed during address witness verification.
///
/// This struct tracks the cryptographic operations performed when verifying
/// address witnesses (P2PKH and P2SH), which are used to calculate processing fees.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddressWitnessVerificationOperations {
    /// Number of ECDSA secp256k1 signature verifications performed.
    /// Each verification involves elliptic curve operations.
    pub ecdsa_signature_verifications: u16,

    /// Number of times we hash the signable bytes using sha256d.
    /// For P2PKH: 1 (hash computed once for signature verification)
    /// For P2SH multisig: 1 (hash computed once, reused for all signatures)
    ///
    /// Note: This is NOT per signature - with P2SH optimization, the message
    /// hash is computed once and reused for all signature verifications.
    pub message_hash_count: u16,

    /// Number of public key hash (Hash160) verifications performed.
    /// Hash160 = RIPEMD160(SHA256(pubkey))
    pub pubkey_hash_verifications: u16,

    /// Number of script hash (SHA256) verifications performed.
    /// Used to verify P2SH redeem scripts.
    pub script_hash_verifications: u16,

    /// Size of the signable bytes in bytes.
    /// Used to calculate the number of SHA256 blocks for the first SHA256.
    /// The second SHA256 (finalization) is always 1 block (32-byte input).
    pub signable_bytes_len: usize,
}

impl AddressWitnessVerificationOperations {
    /// Create a new empty operations tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Operations for a single P2PKH witness verification
    ///
    /// # Arguments
    /// * `signable_bytes_len` - Length of the signable bytes being signed
    pub fn for_p2pkh(signable_bytes_len: usize) -> Self {
        Self {
            ecdsa_signature_verifications: 1,
            message_hash_count: 1,
            pubkey_hash_verifications: 1,
            script_hash_verifications: 0,
            signable_bytes_len,
        }
    }

    /// Operations for a P2SH multisig witness verification
    ///
    /// # Arguments
    /// * `signatures_verified` - Number of signatures that were actually verified
    ///   (may be more than threshold due to signature ordering in CHECKMULTISIG)
    /// * `signable_bytes_len` - Length of the signable bytes being signed
    pub fn for_p2sh_multisig(signatures_verified: u16, signable_bytes_len: usize) -> Self {
        Self {
            ecdsa_signature_verifications: signatures_verified,
            // Hash is computed once and reused for all signature verifications
            message_hash_count: 1,
            pubkey_hash_verifications: 0,
            script_hash_verifications: 1,
            signable_bytes_len,
        }
    }

    /// Combine operations from multiple witness verifications
    pub fn combine(&mut self, other: &Self) {
        self.ecdsa_signature_verifications = self
            .ecdsa_signature_verifications
            .saturating_add(other.ecdsa_signature_verifications);
        self.message_hash_count = self
            .message_hash_count
            .saturating_add(other.message_hash_count);
        self.pubkey_hash_verifications = self
            .pubkey_hash_verifications
            .saturating_add(other.pubkey_hash_verifications);
        self.script_hash_verifications = self
            .script_hash_verifications
            .saturating_add(other.script_hash_verifications);
        // Use the max signable_bytes_len since all signatures sign the same bytes
        self.signable_bytes_len = self.signable_bytes_len.max(other.signable_bytes_len);
    }

    /// Total number of signature verifications
    pub fn total_signature_verifications(&self) -> u16 {
        self.ecdsa_signature_verifications
    }

    /// Total number of hash operations
    pub fn total_hash_operations(&self) -> u16 {
        self.pubkey_hash_verifications
            .saturating_add(self.script_hash_verifications)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2pkh_operations() {
        let ops = AddressWitnessVerificationOperations::for_p2pkh(100);
        assert_eq!(ops.ecdsa_signature_verifications, 1);
        assert_eq!(ops.pubkey_hash_verifications, 1);
        assert_eq!(ops.script_hash_verifications, 0);
        assert_eq!(ops.signable_bytes_len, 100);
    }

    #[test]
    fn test_p2sh_multisig_operations() {
        // 2-of-3 multisig where 2 signatures were verified
        let ops = AddressWitnessVerificationOperations::for_p2sh_multisig(2, 150);
        assert_eq!(ops.ecdsa_signature_verifications, 2);
        assert_eq!(ops.pubkey_hash_verifications, 0);
        assert_eq!(ops.script_hash_verifications, 1);
        assert_eq!(ops.signable_bytes_len, 150);
    }

    #[test]
    fn test_combine_operations() {
        let mut ops1 = AddressWitnessVerificationOperations::for_p2pkh(100);
        let ops2 = AddressWitnessVerificationOperations::for_p2sh_multisig(3, 150);

        ops1.combine(&ops2);

        assert_eq!(ops1.ecdsa_signature_verifications, 4); // 1 + 3
        assert_eq!(ops1.pubkey_hash_verifications, 1); // 1 + 0
        assert_eq!(ops1.script_hash_verifications, 1); // 0 + 1
        assert_eq!(ops1.signable_bytes_len, 150); // max(100, 150)
    }
}
