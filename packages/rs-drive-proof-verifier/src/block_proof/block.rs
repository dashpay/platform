//! The quorum's signature over a block.

use super::header::check_header;
use super::header::header_time_ms;
use super::BlockProofError;
use crate::verify::{quorum_public_key, verify_quorum_signature};
use crate::{ContextProvider, Error};
use dapi_grpc::platform::v0::get_blocks_response::get_blocks_response_v0::Block;
use dapi_grpc::platform::v0::tenderdash_block_proof::{BlockId, Header};
use dapi_grpc::platform::v0::TenderdashBlockProof;
use tenderdash_abci::proto::types::{BlockId as SignedBlockId, Commit, PartSetHeader};
use tenderdash_abci::signatures::Signable;

/// The header and block id of a proof, which a well-formed one always carries.
pub fn proof_parts(proof: &TenderdashBlockProof) -> Result<(&Header, &BlockId), BlockProofError> {
    let header = proof
        .header
        .as_ref()
        .ok_or(BlockProofError::MissingHeader)?;
    let block_id = proof
        .block_id
        .as_ref()
        .ok_or(BlockProofError::MissingBlockId)?;
    Ok((header, block_id))
}

/// Check that the fields a `getBlocks` block repeats from its header are the ones in
/// `proof`. Only the header is signed, and a node reads those fields from another
/// Tenderdash answer than the proof, so nothing else ties them to it.
pub fn check_block_fields(
    proof: &TenderdashBlockProof,
    block: &Block,
) -> Result<(), BlockProofError> {
    let (header, block_id) = proof_parts(proof)?;

    let fields = [
        ("height", block.height == header.height),
        ("hash", block.hash == block_id.hash),
        ("time", block.time_ms == header_time_ms(header)?),
        (
            "core chain locked height",
            block.core_chain_locked_height == header.core_chain_locked_height,
        ),
        ("app hash", block.app_hash == header.app_hash),
        (
            "proposer",
            block.proposer_pro_tx_hash == header.proposer_pro_tx_hash,
        ),
    ];
    match fields.iter().find(|(_, matches)| !matches) {
        Some((field, _)) => Err(BlockProofError::FieldMismatch {
            height: header.height,
            field,
        }),
        None => Ok(()),
    }
}

/// Verify that a quorum committed the block `proof` describes, and return that block's
/// header, every field of which the quorum's signature then covers.
///
/// The header must hash to the block hash and the state id of the proof's block id, and
/// the proof's signature must be the quorum's threshold signature over the precommit vote
/// for that block id at the header's chain, height and the proof's round. The quorum's key
/// comes from `provider`, looked up by the quorum type and hash the proof names and the
/// header's Core height.
///
/// Once the signature verifies, and only then, that Core height is reported to
/// [ContextProvider::core_height_verified]: the state id covers it, so the quorum signed it.
///
/// What this does not show: that the block is recent, or that `provider`'s quorum was the
/// validator set of that height. Any quorum of the named type whose key `provider` returns
/// is accepted, as for every other Platform proof.
pub fn verify_block_proof<'a>(
    proof: &'a TenderdashBlockProof,
    provider: &dyn ContextProvider,
) -> Result<&'a Header, Error> {
    let (header, block_id) = proof_parts(proof)?;
    check_header(header, block_id)?;

    let out_of_range = |field| BlockProofError::VoteFieldOutOfRange {
        height: header.height,
        field,
    };
    let height = i64::try_from(header.height).map_err(|_| out_of_range("height"))?;
    let round = i32::try_from(proof.round).map_err(|_| out_of_range("round"))?;
    let quorum_type = u8::try_from(proof.quorum_type).map_err(|_| out_of_range("quorum type"))?;
    let quorum_hash = <[u8; 32]>::try_from(proof.quorum_hash.as_slice()).map_err(|e| {
        Error::ResponseDecodeError {
            error: "invalid quorum hash size: ".to_string() + &e.to_string(),
        }
    })?;

    let public_key = quorum_public_key(
        provider,
        proof.quorum_type,
        quorum_hash,
        header.core_chain_locked_height,
    )?;

    // The commit Tenderdash stored for the block, as far as its signature covers it
    let commit = Commit {
        height,
        round,
        block_id: Some(SignedBlockId {
            hash: block_id.hash.clone(),
            part_set_header: Some(PartSetHeader {
                total: block_id.part_set_total,
                hash: block_id.part_set_hash.clone(),
            }),
            state_id: block_id.state_id.clone(),
        }),
        quorum_hash: quorum_hash.to_vec(),
        ..Default::default()
    };
    let sign_digest = commit
        .calculate_sign_hash(&header.chain_id, quorum_type, &quorum_hash, height, round)
        .map_err(|e| Error::SignDigestFailed {
            error: e.to_string(),
        })?;

    verify_quorum_signature(
        provider,
        proof.quorum_type,
        quorum_hash,
        header.core_chain_locked_height,
        &public_key,
        &sign_digest,
        &proof.signature,
    )?;

    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::{
        block_id_of, header_of, sign, signed_proof, TestQuorum, CORE_HEIGHT, QUORUM_HASH,
        QUORUM_TYPE,
    };
    use super::*;

    fn proof() -> TenderdashBlockProof {
        signed_proof(header_of(7, &[]))
    }

    /// Verify `proof` against the test quorum, expecting no Core height to be reported.
    fn refused(proof: &TenderdashBlockProof) -> Error {
        let quorum = TestQuorum::new();
        let error = verify_block_proof(proof, &quorum).unwrap_err();
        assert!(quorum.verified.lock().unwrap().is_empty());
        error
    }

    #[test]
    fn should_accept_a_block_the_quorum_signed() {
        let proof = proof();
        let quorum = TestQuorum::new();

        let header = verify_block_proof(&proof, &quorum).unwrap();

        assert_eq!(Some(header), proof.header.as_ref());
        assert_eq!(
            *quorum.verified.lock().unwrap(),
            [(QUORUM_TYPE, QUORUM_HASH, CORE_HEIGHT)]
        );
    }

    #[test]
    fn should_refuse_a_proof_without_its_parts() {
        let mut proof = proof();
        proof.header = None;
        assert!(matches!(
            refused(&proof),
            Error::InvalidBlockProof(BlockProofError::MissingHeader)
        ));

        let mut proof = self::proof();
        proof.block_id = None;
        assert!(matches!(
            refused(&proof),
            Error::InvalidBlockProof(BlockProofError::MissingBlockId)
        ));
    }

    #[test]
    fn should_refuse_a_header_that_does_not_hash_to_the_block_id() {
        let mut proof = proof();
        proof.header.as_mut().unwrap().data_hash[0] ^= 1;

        assert!(matches!(
            refused(&proof),
            Error::InvalidBlockProof(BlockProofError::HeaderHashMismatch { height: 7 })
        ));
    }

    #[test]
    fn should_refuse_a_header_that_does_not_hash_to_the_state_id() {
        // The block hash leaves the Core height out, so only the state id catches it
        let mut proof = proof();
        proof.header.as_mut().unwrap().core_chain_locked_height += 1;
        assert!(matches!(
            refused(&proof),
            Error::InvalidBlockProof(BlockProofError::StateIdMismatch { height: 7 })
        ));

        let mut proof = self::proof();
        proof.block_id.as_mut().unwrap().state_id[0] ^= 1;
        assert!(matches!(
            refused(&proof),
            Error::InvalidBlockProof(BlockProofError::StateIdMismatch { height: 7 })
        ));
    }

    /// A node can change a header and derive a matching block id from it, but not sign it.
    #[test]
    fn should_refuse_a_consistent_header_the_quorum_did_not_sign() {
        let changes: [fn(&mut Header); 4] = [
            |header| header.data_hash[0] ^= 1,
            |header| header.results_hash[0] ^= 1,
            |header| header.core_chain_locked_height += 1,
            |header| header.chain_id.push('x'),
        ];
        for change in changes {
            let mut proof = proof();
            let header = proof.header.as_mut().unwrap();
            change(header);
            proof.block_id = Some(block_id_of(header));

            assert!(matches!(refused(&proof), Error::InvalidSignature { .. }));
        }
    }

    #[test]
    fn should_refuse_the_signature_of_another_block() {
        let mut proof = proof();
        proof.signature = signed_proof(header_of(8, &[])).signature;

        assert!(matches!(refused(&proof), Error::InvalidSignature { .. }));
    }

    #[test]
    fn should_refuse_a_signature_when_what_the_vote_covers_changes() {
        let changes: [fn(&mut TenderdashBlockProof); 3] = [
            |proof| proof.round += 1,
            |proof| proof.block_id.as_mut().unwrap().part_set_total += 1,
            |proof| proof.block_id.as_mut().unwrap().part_set_hash[0] ^= 1,
        ];
        for change in changes {
            let mut proof = proof();
            change(&mut proof);

            assert!(matches!(refused(&proof), Error::InvalidSignature { .. }));
        }
    }

    #[test]
    fn should_refuse_a_signature_of_another_key() {
        let quorum = TestQuorum::with_another_key();

        assert!(matches!(
            verify_block_proof(&proof(), &quorum),
            Err(Error::InvalidSignature { .. })
        ));
        assert!(quorum.verified.lock().unwrap().is_empty());
    }

    #[test]
    fn should_refuse_a_malformed_signature() {
        let mut proof = proof();
        proof.signature[5] ^= 1;
        assert!(matches!(
            refused(&proof),
            Error::InvalidSignature { .. } | Error::SignatureVerificationError { .. }
        ));

        proof.signature = vec![0; 96];
        assert!(matches!(
            refused(&proof),
            Error::SignatureVerificationError { .. }
        ));

        proof.signature = vec![1; 95];
        assert!(matches!(
            refused(&proof),
            Error::InvalidSignatureFormat { .. }
        ));
    }

    #[test]
    fn should_ask_for_the_key_of_the_quorum_the_proof_names() {
        // Signed for the quorum it names, which the provider does not know
        let mut proof = proof();
        proof.quorum_hash = vec![8; 32];
        sign(&mut proof);
        assert!(matches!(
            refused(&proof),
            Error::QuorumKeyUnavailable {
                quorum_type: QUORUM_TYPE,
                quorum_hash,
                core_chain_locked_height: CORE_HEIGHT,
                ..
            } if quorum_hash == [8; 32]
        ));

        let mut proof = self::proof();
        proof.quorum_type = 100;
        sign(&mut proof);
        assert!(matches!(
            refused(&proof),
            Error::QuorumKeyUnavailable {
                quorum_type: 100,
                ..
            }
        ));
    }

    #[test]
    fn should_refuse_fields_no_vote_can_carry() {
        let mut proof = proof();
        proof.quorum_hash.pop();
        assert!(matches!(refused(&proof), Error::ResponseDecodeError { .. }));

        let mut proof = self::proof();
        proof.quorum_type = 256;
        assert!(matches!(
            refused(&proof),
            Error::InvalidBlockProof(BlockProofError::VoteFieldOutOfRange {
                field: "quorum type",
                ..
            })
        ));

        let mut proof = self::proof();
        proof.round = u32::MAX;
        assert!(matches!(
            refused(&proof),
            Error::InvalidBlockProof(BlockProofError::VoteFieldOutOfRange { field: "round", .. })
        ));
    }
}
