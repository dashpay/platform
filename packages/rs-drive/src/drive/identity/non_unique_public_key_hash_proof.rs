//! Proof encodings for the versioned non-unique identity lookup.
use super::identity_and_non_unique_public_key_hash_double_proof::IdentityAndNonUniquePublicKeyHashDoubleProof;
use crate::error::{proof::ProofError, Error};

/// Proof selected by the producer's protocol generation.
#[derive(Clone, Debug)]
pub enum NonUniquePublicKeyHashProof {
    /// Historical separate index and identity proofs.
    Legacy(IdentityAndNonUniquePublicKeyHashDoubleProof),
    /// One complete proof of index selection and identity.
    Current(Vec<u8>),
}
impl NonUniquePublicKeyHashProof {
    /// Returns the legacy encoding, rejecting a mismatched handler generation.
    pub fn into_legacy(self) -> Result<IdentityAndNonUniquePublicKeyHashDoubleProof, Error> {
        match self {
            Self::Legacy(proof) => Ok(proof),
            Self::Current(_) => Err(Error::Proof(ProofError::CorruptedProof(
                "legacy handler requires separate index and identity proofs".into(),
            ))),
        }
    }
    /// Returns the current encoding, rejecting a mismatched handler generation.
    pub fn into_current(self) -> Result<Vec<u8>, Error> {
        match self {
            Self::Current(proof) => Ok(proof),
            Self::Legacy(_) => Err(Error::Proof(ProofError::CorruptedProof(
                "current handler requires a single identity proof".into(),
            ))),
        }
    }
}
/// Explicit wire encoding, independent of the producer's advertised protocol.
#[derive(Clone, Copy, Debug)]
pub enum NonUniquePublicKeyHashProofRef<'a> {
    /// Historical separate index and identity proofs.
    Legacy(&'a IdentityAndNonUniquePublicKeyHashDoubleProof),
    /// One complete proof of index selection and identity.
    Current(&'a [u8]),
}
impl<'a> From<&'a NonUniquePublicKeyHashProof> for NonUniquePublicKeyHashProofRef<'a> {
    fn from(proof: &'a NonUniquePublicKeyHashProof) -> Self {
        match proof {
            NonUniquePublicKeyHashProof::Legacy(proof) => Self::Legacy(proof),
            NonUniquePublicKeyHashProof::Current(proof) => Self::Current(proof),
        }
    }
}
impl<'a> From<&'a IdentityAndNonUniquePublicKeyHashDoubleProof>
    for NonUniquePublicKeyHashProofRef<'a>
{
    fn from(proof: &'a IdentityAndNonUniquePublicKeyHashDoubleProof) -> Self {
        Self::Legacy(proof)
    }
}
impl<'a> From<&'a [u8]> for NonUniquePublicKeyHashProofRef<'a> {
    fn from(proof: &'a [u8]) -> Self {
        Self::Current(proof)
    }
}
impl<'a> From<&'a Vec<u8>> for NonUniquePublicKeyHashProofRef<'a> {
    fn from(proof: &'a Vec<u8>) -> Self {
        Self::Current(proof)
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for NonUniquePublicKeyHashProofRef<'a> {
    fn from(proof: &'a [u8; N]) -> Self {
        Self::Current(proof)
    }
}
