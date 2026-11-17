//! The two hashes Tenderdash derives from a block header and a quorum signs: the block
//! hash and the state id.

use super::merkle::merkle_root;
use super::BlockProofError;
use dapi_grpc::platform::v0::tenderdash_block_proof::{BlockId, Header};
use dapi_grpc::Message;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::{BlockId as HashedBlockId, PartSetHeader, StateId};
use tenderdash_abci::proto::version::Consensus;
use tenderdash_abci::signatures::Hashable;

/// Tenderdash hashes an absent app hash into a state id as this many zero bytes.
const APP_HASH_SIZE: usize = 32;

/// The hash Tenderdash gives a block: the Merkle root of its header's fields, each in its
/// protobuf encoding. A string, number or hash is wrapped in a one-field message first, so
/// one that is empty or zero is an empty leaf.
///
/// The core chain locked height is an empty leaf whatever its value: the state id is what
/// commits to it.
pub fn header_hash(header: &Header) -> [u8; 32] {
    let last_block_id = header.last_block_id.clone().unwrap_or_default();

    merkle_root(&[
        Consensus {
            block: header.block_version,
            app: header.app_version,
        }
        .encode_to_vec(),
        header.chain_id.encode_to_vec(),
        header.height.encode_to_vec(),
        Vec::new(),
        Timestamp {
            seconds: header.time_seconds,
            nanos: header.time_nanos,
        }
        .encode_to_vec(),
        HashedBlockId {
            hash: last_block_id.hash,
            part_set_header: Some(PartSetHeader {
                total: last_block_id.part_set_total,
                hash: last_block_id.part_set_hash,
            }),
            state_id: last_block_id.state_id,
        }
        .encode_to_vec(),
        header.last_commit_hash.encode_to_vec(),
        header.data_hash.encode_to_vec(),
        header.validators_hash.encode_to_vec(),
        header.next_validators_hash.encode_to_vec(),
        header.consensus_hash.encode_to_vec(),
        header.next_consensus_hash.encode_to_vec(),
        header.app_hash.encode_to_vec(),
        header.results_hash.encode_to_vec(),
        header.evidence_hash.encode_to_vec(),
        header.proposer_pro_tx_hash.encode_to_vec(),
        header.proposed_app_version.encode_to_vec(),
    ])
}

/// A header's time in milliseconds since the Unix epoch, the precision a state id keeps.
pub fn header_time_ms(header: &Header) -> Result<u64, BlockProofError> {
    header
        .time_seconds
        .checked_mul(1000)
        .and_then(|ms| ms.checked_add(i64::from(header.time_nanos / 1_000_000)))
        .and_then(|ms| u64::try_from(ms).ok())
        .ok_or(BlockProofError::TimeOutOfRange {
            height: header.height,
        })
}

/// The state id Tenderdash derives from a header, which the quorum signs next to the
/// block hash: the hash of these fields as Tenderdash's `StateID`.
pub fn state_id_hash(header: &Header) -> Result<[u8; 32], BlockProofError> {
    let app_hash = if header.app_hash.is_empty() {
        vec![0; APP_HASH_SIZE]
    } else {
        header.app_hash.clone()
    };
    let state_id = StateId {
        app_version: header.app_version,
        height: header.height,
        app_hash,
        core_chain_locked_height: header.core_chain_locked_height,
        time: header_time_ms(header)?,
    };

    // A state id is hashed the same way whatever the chain, height and round
    state_id
        .calculate_msg_hash(&header.chain_id, 0, 0)
        .ok()
        .and_then(|hash| <[u8; 32]>::try_from(hash).ok())
        .ok_or(BlockProofError::StateIdNotHashed {
            height: header.height,
        })
}

/// Check that `header` is the one behind `block_id`: that it hashes to the block hash and
/// to the state id, the two values of a block id a quorum's signature covers.
pub fn check_header(header: &Header, block_id: &BlockId) -> Result<(), BlockProofError> {
    if header_hash(header).as_slice() != block_id.hash {
        return Err(BlockProofError::HeaderHashMismatch {
            height: header.height,
        });
    }
    if state_id_hash(header)?.as_slice() != block_id.state_id {
        return Err(BlockProofError::StateIdMismatch {
            height: header.height,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// The header of the "Generates expected hash" case of `TestHeaderHash` in Tenderdash's
    /// `types/block_test.go`, which sets every field.
    fn tenderdash_header() -> Header {
        let hash_of = |seed: &str| Sha256::digest(seed).to_vec();
        let last_block = Header {
            app_version: 1,
            height: 3,
            core_chain_locked_height: 1,
            // 2022-03-04T05:06:07.000000008Z
            time_seconds: 1_646_370_367,
            time_nanos: 8,
            app_hash: hash_of("app_hash"),
            ..Default::default()
        };

        Header {
            block_version: 1,
            app_version: 2,
            chain_id: "chainId".to_string(),
            height: 3,
            core_chain_locked_height: 1,
            // 2019-10-13T16:14:44Z
            time_seconds: 1_570_983_284,
            time_nanos: 0,
            last_block_id: Some(BlockId {
                hash: vec![0; 32],
                part_set_total: 6,
                part_set_hash: vec![0; 32],
                state_id: state_id_hash(&last_block).unwrap().to_vec(),
            }),
            last_commit_hash: hash_of("last_commit_hash"),
            data_hash: hash_of("data_hash"),
            validators_hash: hash_of("validators_hash"),
            next_validators_hash: hash_of("next_validators_hash"),
            consensus_hash: hash_of("consensus_hash"),
            next_consensus_hash: hash_of("next_consensus_hash"),
            app_hash: hash_of("app_hash"),
            results_hash: hash_of("last_results_hash"),
            evidence_hash: hash_of("evidence_hash"),
            proposer_pro_tx_hash: hash_of("proposer_pro_tx_hash"),
            proposed_app_version: 1,
        }
    }

    fn block_id_of(header: &Header) -> BlockId {
        BlockId {
            hash: header_hash(header).to_vec(),
            part_set_total: 1,
            part_set_hash: vec![0xdd; 32],
            state_id: state_id_hash(header).unwrap().to_vec(),
        }
    }

    #[test]
    fn should_hash_a_header_as_tenderdash_does() {
        assert_eq!(
            hex::encode(header_hash(&tenderdash_header())),
            "ff24ddab9e1550beb40ab7ad432a4d577560e9b87a80c7bb86e75263974b87e0"
        );
    }

    /// `TestHeaderHashVector` of the same file: no last block, a time before 1970 with
    /// nanoseconds, and hashes that are the text of a hex string.
    #[test]
    fn should_hash_a_header_with_empty_fields_as_tenderdash_does() {
        let text = b"f2564c78071e26643ae9b3e2a19fa0dc10d4d9e873aa0be808660123f11a1e78".to_vec();
        let header = Header {
            block_version: 1,
            app_version: 1,
            chain_id: "test".to_string(),
            height: 50,
            // what Go's `time.Date(math.MaxInt64, 0, 0, 0, 0, 0, math.MaxInt64, time.UTC)`
            // wraps around to: 0291-03-10T13:13:40.854775807Z
            time_seconds: -52_978_185_980,
            time_nanos: 854_775_807,
            last_block_id: Some(BlockId::default()),
            last_commit_hash: text.clone(),
            data_hash: text.clone(),
            validators_hash: text.clone(),
            next_validators_hash: text.clone(),
            consensus_hash: text.clone(),
            next_consensus_hash: text.clone(),
            app_hash: text.clone(),
            results_hash: text.clone(),
            evidence_hash: text.clone(),
            proposer_pro_tx_hash: text,
            ..Default::default()
        };

        assert_eq!(
            hex::encode(header_hash(&header)),
            "1476e858bfe231d7cd767a9693f63fe6a8757e8c07aba432f338a5ecba2b0342"
        );
    }

    #[test]
    fn should_leave_the_core_chain_locked_height_to_the_state_id() {
        let header = tenderdash_header();
        let mut other = header.clone();
        other.core_chain_locked_height += 1;

        assert_eq!(header_hash(&other), header_hash(&header));
        assert_ne!(
            state_id_hash(&other).unwrap(),
            state_id_hash(&header).unwrap()
        );
    }

    #[test]
    fn should_hash_an_absent_app_hash_into_the_state_id_as_zeros() {
        let mut header = tenderdash_header();
        header.app_hash = Vec::new();
        let mut zeros = header.clone();
        zeros.app_hash = vec![0; 32];

        assert_eq!(
            state_id_hash(&header).unwrap(),
            state_id_hash(&zeros).unwrap()
        );
    }

    #[test]
    fn should_keep_milliseconds_of_the_time_in_the_state_id() {
        let mut header = tenderdash_header();
        header.time_nanos = 123_999_999;

        assert_eq!(header_time_ms(&header), Ok(1_570_983_284_123));
    }

    #[test]
    fn should_refuse_a_state_id_for_a_time_before_1970() {
        let mut header = tenderdash_header();
        header.time_seconds = -1;

        assert_eq!(
            state_id_hash(&header),
            Err(BlockProofError::TimeOutOfRange { height: 3 })
        );
    }

    #[test]
    fn should_accept_a_header_that_hashes_to_its_block_id() {
        let header = tenderdash_header();

        assert_eq!(check_header(&header, &block_id_of(&header)), Ok(()));
    }

    /// The block hash leaves the core chain locked height out, so that field is the one
    /// only the state id catches.
    #[test]
    fn should_tell_which_hash_every_header_field_changes() {
        let signed = tenderdash_header();
        let block_id = block_id_of(&signed);
        let block_hash = Err(BlockProofError::HeaderHashMismatch { height: 3 });
        type Change = (&'static str, fn(&mut Header));
        let changes: [Change; 18] = [
            ("block_version", |header| header.block_version += 1),
            ("app_version", |header| header.app_version += 1),
            ("chain_id", |header| header.chain_id.push('x')),
            ("time_seconds", |header| header.time_seconds += 1),
            ("time_nanos", |header| header.time_nanos += 1),
            ("last_block_id", |header| {
                header.last_block_id.as_mut().unwrap().part_set_total += 1
            }),
            ("last_commit_hash", |header| header.last_commit_hash[0] ^= 1),
            ("data_hash", |header| header.data_hash[0] ^= 1),
            ("validators_hash", |header| header.validators_hash[0] ^= 1),
            ("next_validators_hash", |header| {
                header.next_validators_hash[0] ^= 1
            }),
            ("consensus_hash", |header| header.consensus_hash[0] ^= 1),
            ("next_consensus_hash", |header| {
                header.next_consensus_hash[0] ^= 1
            }),
            ("app_hash", |header| header.app_hash[0] ^= 1),
            ("results_hash", |header| header.results_hash[0] ^= 1),
            ("evidence_hash", |header| header.evidence_hash[0] ^= 1),
            ("proposer_pro_tx_hash", |header| {
                header.proposer_pro_tx_hash[0] ^= 1
            }),
            ("proposed_app_version", |header| {
                header.proposed_app_version += 1
            }),
            ("last_block_id.state_id", |header| {
                header.last_block_id.as_mut().unwrap().state_id[0] ^= 1
            }),
        ];

        for (field, change) in changes {
            let mut header = signed.clone();
            change(&mut header);
            assert_eq!(check_header(&header, &block_id), block_hash, "{field}");
        }

        let mut header = signed.clone();
        header.core_chain_locked_height += 1;
        assert_eq!(
            check_header(&header, &block_id),
            Err(BlockProofError::StateIdMismatch { height: 3 })
        );
    }

    #[test]
    fn should_refuse_a_header_under_another_height() {
        let signed = tenderdash_header();
        let block_id = block_id_of(&signed);
        let mut header = signed;
        header.height += 1;

        assert_eq!(
            check_header(&header, &block_id),
            Err(BlockProofError::HeaderHashMismatch { height: 4 })
        );
    }
}
