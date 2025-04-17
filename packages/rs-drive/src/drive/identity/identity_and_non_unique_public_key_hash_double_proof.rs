/// Represents a proof containing an optional identity proof and a required
/// proof for the identity ID and non-unique public key hash.
///
/// This struct is used to verify the authenticity and validity of an identity
/// and its associated non-unique public key hash.
///
/// # Fields
///
/// * `identity_proof` - An optional proof for the identity, represented as a
///   serialized byte vector. This may be `None` if no additional proof is required.
/// * `identity_id_public_key_hash_proof` - A required proof verifying the
///   association between an identity ID and its non-unique public key hash,
///   stored as a serialized byte vector.
pub struct IdentityAndNonUniquePublicKeyHashDoubleProof {
    /// Optional proof of identity, stored as a serialized byte vector.
    pub identity_proof: Option<Vec<u8>>,

    /// Proof linking an identity ID to a non-unique public key hash,
    /// stored as a serialized byte vector.
    pub identity_id_public_key_hash_proof: Vec<u8>,
}
