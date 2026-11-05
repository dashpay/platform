//! Retired: quorum storage version 1, tag 1 of `SignatureVerificationQuorumSetForSaving`.
//!
//! The reader was removed in Platform v5.1.0 by #5323, to drop the C++ `bls-signatures`
//! dependency from the default production node: the reader decoded this format's previous quorums
//! through that crate. Platform v4.0.0 is the upgrade path, as it reads this format and writes a
//! supported one. Only the placeholder remains, and it keeps the tag reserved, so the tags of the
//! supported formats do not move. The quorum types below it are live: version 2 stores them.

use super::public_key_for_saving::PublicKeyForSaving;
use crate::platform_types::signature_verification_quorum_set::v0::quorum_set::PreviousPastQuorumsV0;
use crate::platform_types::signature_verification_quorum_set::{Quorums, VerificationQuorum};
use crate::platform_types::unsupported_legacy_bls_storage::unsupported_legacy_bls_storage_error;
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};

use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::platform_value::Bytes32;

/// Has no values: nothing can write this format, and decoding it always fails.
#[derive(Debug, Clone)]
pub enum SignatureVerificationQuorumSetForSavingV1 {}

impl Encode for SignatureVerificationQuorumSetForSavingV1 {
    fn encode<E: Encoder>(&self, _encoder: &mut E) -> Result<(), EncodeError> {
        match *self {}
    }
}

impl<Context> Decode<Context> for SignatureVerificationQuorumSetForSavingV1 {
    fn decode<D: Decoder<Context = Context>>(_decoder: &mut D) -> Result<Self, DecodeError> {
        Err(unsupported_legacy_bls_storage_error())
    }
}

bincode::impl_borrow_decode!(SignatureVerificationQuorumSetForSavingV1);

#[derive(Debug, Clone, Encode, Decode)]
pub struct QuorumForSavingV1 {
    hash: Bytes32,
    // Preserve the shipped 48-byte key layout in every selecting protocol version.
    public_key: PublicKeyForSaving,
    index: Option<u32>,
}

impl From<Vec<QuorumForSavingV1>> for Quorums<VerificationQuorum> {
    fn from(value: Vec<QuorumForSavingV1>) -> Self {
        Quorums::from_iter(value.into_iter().map(|quorum| {
            (
                QuorumHash::from_byte_array(quorum.hash.to_buffer()),
                VerificationQuorum {
                    public_key: quorum.public_key.0,
                    index: quorum.index,
                },
            )
        }))
    }
}
impl From<Quorums<VerificationQuorum>> for Vec<QuorumForSavingV1> {
    fn from(quorums: Quorums<VerificationQuorum>) -> Self {
        quorums
            .into_iter()
            .map(|(hash, quorum)| QuorumForSavingV1 {
                hash: Bytes32::from(hash.as_byte_array()),
                public_key: PublicKeyForSaving(quorum.public_key),
                index: quorum.index,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct PreviousPastQuorumsForSavingV1 {
    quorums: Vec<QuorumForSavingV1>,
    last_active_core_height: u32,
    updated_at_core_height: u32,
    previous_change_height: Option<u32>,
}

impl From<PreviousPastQuorumsV0> for PreviousPastQuorumsForSavingV1 {
    fn from(value: PreviousPastQuorumsV0) -> Self {
        let PreviousPastQuorumsV0 {
            quorums,
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        }
    }
}

impl From<PreviousPastQuorumsForSavingV1> for PreviousPastQuorumsV0 {
    fn from(value: PreviousPastQuorumsForSavingV1) -> Self {
        let PreviousPastQuorumsForSavingV1 {
            quorums,
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        }
    }
}
