//! Frozen V019 binary codec. All record and leaf serde implementations come
//! from the immutable dependency revisions in this crate's manifest. Runtime
//! wallet upgrades cannot change them. Only primitive bytes cross this boundary.

use dashcore::hashes::Hash;
use key_wallet::managed_account::transaction_record::TransactionRecord;

/// V019's allocation and record-size bound.
pub const MAX_RECORD_BYTES: usize = 16 * 1024 * 1024;

/// Historical decoder failures, independent of the runtime wallet's types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The record or its declared allocations exceed V019's bound.
    #[error("V019 record exceeds the allocation limit")]
    TooLarge,
    /// The historical binary format could not be decoded.
    #[error("cannot decode V019 transaction record: {0}")]
    Decode(#[source] bincode::error::DecodeError),
    /// The historical input key could not be encoded.
    #[error("cannot encode V019 input outpoint: {0}")]
    Encode(#[from] bincode::error::EncodeError),
    /// Extra bytes follow a complete historical record.
    #[error("unexpected trailing bytes in V019 transaction record")]
    TrailingBytes,
    /// The stored txid differs from its transaction's consensus hash.
    #[error("V019 record txid disagrees with its transaction body hash")]
    TxidMismatch,
}

/// The only projection needed by V020, with no runtime or historical wallet types.
#[derive(Debug, PartialEq, Eq)]
pub struct DecodedRecord {
    /// Internal byte order of the verified transaction ID.
    pub txid: [u8; 32],
    /// Input outpoints encoded in V019's persisted binary key format.
    pub inputs: Vec<Vec<u8>>,
}

/// Decode and validate a complete V019 record with the historical types.
fn decode_record(payload: &[u8]) -> Result<TransactionRecord, Error> {
    if payload.len() > MAX_RECORD_BYTES {
        return Err(Error::TooLarge);
    }
    let config = bincode::config::standard().with_limit::<MAX_RECORD_BYTES>();
    let (record, consumed): (TransactionRecord, usize) =
        bincode::serde::decode_from_slice(payload, config).map_err(|error| match error {
            bincode::error::DecodeError::LimitExceeded => Error::TooLarge,
            error => Error::Decode(error),
        })?;
    if consumed != payload.len() {
        return Err(Error::TrailingBytes);
    }
    if record.txid != record.transaction.txid() {
        return Err(Error::TxidMismatch);
    }
    Ok(record)
}

/// Decode and validate a complete V019 record, then extract its input index.
pub fn decode(payload: &[u8]) -> Result<DecodedRecord, Error> {
    let record = decode_record(payload)?;
    let config = bincode::config::standard().with_limit::<MAX_RECORD_BYTES>();
    let inputs = record
        .transaction
        .input
        .iter()
        .map(|input| bincode::serde::encode_to_vec(input.previous_output, config))
        .collect::<Result<_, _>>()?;
    Ok(DecodedRecord {
        txid: record.txid.to_byte_array(),
        inputs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::transaction_checking::TransactionContext;

    #[test]
    fn should_reject_corrupt_v019_records_without_runtime_decoders() {
        let fields: Vec<_> = include_str!("../../tests/fixtures/v019_kept_records.txt")
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .collect();
        let bytes = hex::decode(fields[3]).unwrap();
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(decode(&trailing), Err(Error::TrailingBytes)));
        assert!(matches!(
            decode(&bytes[..bytes.len() - 1]),
            Err(Error::Decode(_))
        ));
        let txid = hex::decode(fields[1]).unwrap();
        let index = bytes.windows(32).position(|window| window == txid).unwrap();
        let mut mismatched = bytes;
        mismatched[index] ^= 1;
        assert!(matches!(decode(&mismatched), Err(Error::TxidMismatch)));
        assert!(matches!(
            decode(&vec![0; MAX_RECORD_BYTES + 1]),
            Err(Error::TooLarge)
        ));
    }

    #[test]
    fn should_decode_immutable_unmined_contexts_and_leaf_values() {
        for line in include_str!("../../tests/fixtures/v019_kept_records.txt").lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            let bytes = hex::decode(fields[3]).unwrap();
            let record = decode_record(&bytes).unwrap();
            assert!(
                record.block_info().is_none(),
                "height NULL must match the context"
            );
            match (&record.context, fields[0]) {
                (TransactionContext::Mempool, "mempool") => {}
                (TransactionContext::InstantSend(lock), "instant_send") => {
                    assert_eq!(lock.txid, record.txid);
                    assert_eq!(
                        lock.inputs,
                        vec![record.transaction.input[0].previous_output]
                    );
                }
                _ => panic!("wrong fixture context"),
            }
            assert_eq!(
                record.transaction.input[0]
                    .previous_output
                    .txid
                    .to_byte_array(),
                [0x42; 32]
            );
            assert_eq!(record.transaction.input[0].previous_output.vout, 7);
            assert_eq!(
                record.input_details[0].address.to_string(),
                "yMsgnH1xKGa85n4bq2imrZbG2KgrmGttAV"
            );
            assert_eq!(record.transaction.input[0].script_sig.as_bytes(), &[0x51]);
            assert_eq!(
                record.transaction.input[0].witness.to_vec(),
                vec![vec![1, 2, 3]]
            );
            assert_eq!(
                bincode::serde::encode_to_vec(&record, bincode::config::standard()).unwrap(),
                bytes
            );
            let projected = decode(&bytes).unwrap();
            assert_eq!(hex::encode(projected.txid), fields[1]);
            assert_eq!(hex::encode(&projected.inputs[0]), fields[2]);
        }
    }
}
