//! The transactions and payloads a proof carries: quorum commitments, the
//! special transactions that mine them, and coinbases. Each parser rejects
//! everything Core's proof verifier rejects, and trailing data. It is stricter
//! than Core only for legacy-scheme BLS signature bytes Core would read as
//! null (see `check_signature_encoding`).
use crate::bls::{check_signature_encoding, public_key};
use crate::wire::{Hash, Reader, Result};
use crate::Error;

/// Largest transaction a proof may carry, as in Core.
pub const MAX_TRANSACTION_BYTES: usize = 100_000;
/// Largest serialized quorum commitment, as in Core.
pub(crate) const MAX_COMMITMENT_BYTES: usize = 1024;
/// Core's `MAX_SIZE`, the bound its CompactSize reader enforces.
const MAX_COMPACT_SIZE: u64 = 0x0200_0000;

const TRANSACTION_COINBASE: u16 = 5;
const TRANSACTION_QUORUM_COMMITMENT: u16 = 6;
/// First coinbase payload version with ChainLock and credit-pool fields.
const CBTX_CLSIG_AND_BALANCE: u16 = 3;
/// First coinbase payload version with `merkleRootAssetUnlocks` (Core v24).
const CBTX_MERKLE_ROOT_ASSET_UNLOCKS: u16 = 4;

/// The fields of a final quorum commitment a proof needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commitment {
    /// LLMQ type.
    pub quorum_type: u8,
    /// Quorum hash, wire order (the reverse of Platform's and Core RPC's order).
    pub quorum_hash: Hash,
    /// Aggregated quorum public key, a validated Basic-scheme key.
    pub public_key: [u8; 48],
}

/// Size and signing threshold of each LLMQ type a commitment may have, and
/// whether the type rotates (and so carries a quorum index).
fn quorum_parameters(quorum_type: u8) -> Result<(usize, u32, bool)> {
    Ok(match quorum_type {
        1 => (50, 30, false),   // LLMQ_50_60
        2 => (400, 240, false), // LLMQ_400_60
        3 => (400, 340, false), // LLMQ_400_85
        4 => (100, 67, false),  // LLMQ_100_67
        5 => (60, 45, true),    // LLMQ_60_75
        6 => (25, 17, false),   // LLMQ_25_67
        _ => return Err(Error("unsupported quorum type")),
    })
}

/// Parse a serialized `CFinalCommitment` of the Basic BLS era (version 3, or
/// 4 for rotated quorums) that is canonical, non-null and signed by at least
/// the quorum's threshold.
pub fn parse_commitment(bytes: &[u8]) -> Result<Commitment> {
    if bytes.len() > MAX_COMMITMENT_BYTES {
        return Err(Error("commitment size"));
    }
    let mut r = Reader::new(bytes);
    let version = r.u16()?;
    if version != 3 && version != 4 {
        return Err(Error("unsupported commitment version"));
    }
    let quorum_type = r.u8()?;
    let (size, threshold, rotated) = quorum_parameters(quorum_type)?;
    if (version == 4) != rotated {
        return Err(Error("commitment rotation version"));
    }
    let quorum_hash = r.array()?;
    // Core reads the index as int16; negative values fail this check too.
    if rotated && r.u16()? >= 32 {
        return Err(Error("quorum index"));
    }
    // Signers, then valid members: bit count, then packed bits.
    for _ in 0..2 {
        if r.compact(MAX_COMPACT_SIZE)? != size {
            return Err(Error("quorum bit count"));
        }
        let packed = r.take(size.div_ceil(8))?;
        if size % 8 != 0 && packed[packed.len() - 1] >> (size % 8) != 0 {
            return Err(Error("bitset padding"));
        }
        if packed.iter().map(|b| b.count_ones()).sum::<u32>() < threshold {
            return Err(Error("null/undersized quorum"));
        }
    }
    let public_key_bytes = r.array()?;
    public_key(&public_key_bytes)?;
    if quorum_hash == [0; 32] {
        return Err(Error("null quorum"));
    }
    r.take(32)?; // verification vector hash
    check_signature_encoding(&r.array()?)?; // quorum signature
    check_signature_encoding(&r.array()?)?; // members' signature
    r.finish()?;
    Ok(Commitment {
        quorum_type,
        quorum_hash,
        public_key: public_key_bytes,
    })
}

/// Read a special transaction's version and type. Core's `nVersion` is an
/// int16, so 0x8000..=0xFFFF are negative and never special.
fn special_type(r: &mut Reader<'_>) -> Result<Option<u16>> {
    let version = r.u16()? as i16;
    let kind = r.u16()?;
    Ok((version >= 3).then_some(kind))
}

/// A transaction's inputs, outputs and lock time, as far as the parsers need.
struct Body<'a> {
    inputs: usize,
    outputs: usize,
    /// The first input's prevout and scriptSig.
    first_input: Option<(&'a [u8], &'a [u8])>,
}

/// Read a CompactSize-prefixed byte string, at most a transaction long.
fn var_bytes<'a>(r: &mut Reader<'a>) -> Result<&'a [u8]> {
    let len = r.compact(MAX_TRANSACTION_BYTES as u64)?;
    r.take(len)
}

/// Read past a transaction's inputs, outputs and lock time.
fn read_body<'a>(r: &mut Reader<'a>) -> Result<Body<'a>> {
    let max = MAX_TRANSACTION_BYTES as u64;
    let inputs = r.compact(max)?;
    let mut first_input = None;
    for _ in 0..inputs {
        let prevout = r.take(36)?;
        let script = var_bytes(r)?;
        r.u32()?; // sequence
        first_input.get_or_insert((prevout, script));
    }
    let outputs = r.compact(max)?;
    for _ in 0..outputs {
        r.u64()?; // value
        var_bytes(r)?; // scriptPubKey
    }
    r.u32()?; // lock time
    Ok(Body {
        inputs,
        outputs,
        first_input,
    })
}

/// Parse a mined quorum commitment transaction, mined at `height`.
///
/// Consensus requires only a special version, type 6, and a version 1
/// payload carrying the mining height and the commitment; its inputs, outputs
/// and lock time are unconstrained, so any shape is accepted.
pub(crate) fn mined_commitment(bytes: &[u8], height: u32) -> Result<Commitment> {
    let mut r = Reader::new(bytes);
    if special_type(&mut r)? != Some(TRANSACTION_QUORUM_COMMITMENT) {
        return Err(Error("quorum transaction envelope"));
    }
    read_body(&mut r)?;
    let payload = var_bytes(&mut r)?;
    r.finish()?;
    let mut header = Reader::new(payload);
    if header.u16()? != 1 || header.u32()? != height {
        return Err(Error("quorum payload height/version"));
    }
    parse_commitment(&payload[6..])
}

/// Parse the coinbase of the block at `height` and return its masternode and
/// quorum Merkle roots.
///
/// Mirrors Core's proof verifier: any special version, type 5, one null-prevout
/// input with a 1..=100 byte scriptSig, at least one output, and a payload of
/// version 3 or later for `height` with a non-null quorum root and a ChainLock
/// height difference below `height`. Version 4 payloads (Core v24) also carry
/// `merkleRootAssetUnlocks`.
pub fn coinbase_roots(bytes: &[u8], height: u32) -> Result<(Hash, Hash)> {
    if bytes.len() > MAX_TRANSACTION_BYTES {
        return Err(Error("coinbase size"));
    }
    // Core compares the payload's int32 height.
    if height > i32::MAX as u32 {
        return Err(Error("coinbase payload version/height"));
    }
    let mut r = Reader::new(bytes);
    if special_type(&mut r)? != Some(TRANSACTION_COINBASE) {
        return Err(Error("coinbase transaction type/version"));
    }
    let body = read_body(&mut r)?;
    let null_prevout = |prevout: &[u8]| prevout[..32] == [0; 32] && prevout[32..] == [0xff; 4];
    match body.first_input {
        Some((prevout, script))
            if body.inputs == 1 && null_prevout(prevout) && (1..=100).contains(&script.len()) => {}
        _ => return Err(Error("coinbase input")),
    }
    if body.outputs == 0 {
        return Err(Error("coinbase outputs"));
    }
    let mut payload = Reader::new(var_bytes(&mut r)?);
    r.finish()?;

    let version = payload.u16()?;
    if version < CBTX_CLSIG_AND_BALANCE || payload.u32()? != height {
        return Err(Error("coinbase payload version/height"));
    }
    let masternode_root = payload.array()?;
    let quorum_root: Hash = payload.array()?;
    if quorum_root == [0; 32] {
        return Err(Error("coinbase quorum root"));
    }
    if payload.compact(MAX_COMPACT_SIZE)? >= height as usize {
        return Err(Error("coinbase ChainLock height"));
    }
    check_signature_encoding(&payload.array()?)?; // best ChainLock signature
    payload.u64()?; // credit pool balance
    if version >= CBTX_MERKLE_ROOT_ASSET_UNLOCKS {
        payload.take(32)?;
    }
    payload.finish()?;
    Ok((masternode_root, quorum_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::QuorumProofChain;

    /// The testnet fixture's chain, from a real `getquorumproofchain` call.
    fn chain() -> QuorumProofChain {
        let value: serde_json::Value =
            serde_json::from_str(include_str!("../tests/data/testnet.json")).unwrap();
        let proof = hex::decode(value["proof_hex"].as_str().unwrap()).unwrap();
        QuorumProofChain::decode(&proof).unwrap()
    }

    fn compact(out: &mut Vec<u8>, n: usize) {
        match n {
            0..=252 => out.push(n as u8),
            253..=0xffff => {
                out.push(253);
                out.extend((n as u16).to_le_bytes());
            }
            _ => unreachable!("test helper encodes at most u16::MAX"),
        }
    }

    /// A transaction's parts, to re-serialize it with changes.
    struct Parts {
        input: Vec<u8>,
        script: Vec<u8>,
        sequence: u32,
        lock_time: u32,
        payload: Vec<u8>,
    }

    /// Split a transaction with exactly one input (any number of outputs).
    fn split(tx: &[u8]) -> Parts {
        let mut r = Reader::new(tx);
        r.take(4).unwrap();
        assert_eq!(r.compact(1).unwrap(), 1);
        let input = r.take(36).unwrap().to_vec();
        let n = r.compact(100).unwrap();
        let script = r.take(n).unwrap().to_vec();
        let sequence = r.u32().unwrap();
        for _ in 0..r.compact(100_000).unwrap() {
            r.u64().unwrap();
            let n = r.compact(100_000).unwrap();
            r.take(n).unwrap();
        }
        let lock_time = r.u32().unwrap();
        let n = r.compact(100_000).unwrap();
        let payload = r.take(n).unwrap().to_vec();
        r.finish().unwrap();
        Parts {
            input,
            script,
            sequence,
            lock_time,
            payload,
        }
    }

    fn coinbase(parts: &Parts, version: u16, outputs: usize, payload: &[u8]) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.extend(version.to_le_bytes());
        tx.extend(TRANSACTION_COINBASE.to_le_bytes());
        compact(&mut tx, 1);
        tx.extend(&parts.input);
        compact(&mut tx, parts.script.len());
        tx.extend(&parts.script);
        tx.extend(parts.sequence.to_le_bytes());
        compact(&mut tx, outputs);
        for _ in 0..outputs {
            tx.extend(0u64.to_le_bytes());
            compact(&mut tx, 0);
        }
        tx.extend(parts.lock_time.to_le_bytes());
        compact(&mut tx, payload.len());
        tx.extend(payload);
        tx
    }

    #[test]
    fn should_read_v24_coinbase_payloads_with_the_asset_unlock_root() {
        let chain = chain();
        let height = chain.target.height;
        let expected = coinbase_roots(&chain.coinbase.transaction, height).unwrap();
        let parts = split(&chain.coinbase.transaction);
        assert_eq!(parts.payload[..2], CBTX_CLSIG_AND_BALANCE.to_le_bytes());

        let mut v4 = parts.payload.clone();
        v4[..2].copy_from_slice(&CBTX_MERKLE_ROOT_ASSET_UNLOCKS.to_le_bytes());
        v4.extend([0x5a; 32]);
        assert_eq!(
            coinbase_roots(&coinbase(&parts, 3, 1, &v4), height),
            Ok(expected)
        );

        // A v3 payload followed by 32 bytes, and a v4 payload without them,
        // are not what Core deserializes.
        let mut v3_long = parts.payload.clone();
        v3_long.extend([0x5a; 32]);
        assert_eq!(
            coinbase_roots(&coinbase(&parts, 3, 1, &v3_long), height),
            Err(Error("trailing bytes"))
        );
        let mut v4_short = parts.payload.clone();
        v4_short[..2].copy_from_slice(&CBTX_MERKLE_ROOT_ASSET_UNLOCKS.to_le_bytes());
        assert_eq!(
            coinbase_roots(&coinbase(&parts, 3, 1, &v4_short), height),
            Err(Error("truncated input"))
        );
    }

    #[test]
    fn should_accept_every_coinbase_shape_consensus_allows() {
        let chain = chain();
        let height = chain.target.height;
        let expected = coinbase_roots(&chain.coinbase.transaction, height).unwrap();
        let parts = split(&chain.coinbase.transaction);
        // Any special version, and more than 4,096 outputs.
        let wide = coinbase(&parts, 4, 4097, &parts.payload);
        assert_eq!(coinbase_roots(&wide, height), Ok(expected));

        let rejected = |tx: &[u8]| coinbase_roots(tx, height).unwrap_err();
        assert_eq!(
            rejected(&coinbase(&parts, 2, 1, &parts.payload)),
            Error("coinbase transaction type/version")
        );
        assert_eq!(
            rejected(&coinbase(&parts, 0xfffe, 1, &parts.payload)),
            Error("coinbase transaction type/version")
        );
        assert_eq!(
            rejected(&coinbase(&parts, 3, 0, &parts.payload)),
            Error("coinbase outputs")
        );
        assert_eq!(
            coinbase_roots(&coinbase(&parts, 3, 1, &parts.payload), height + 1).unwrap_err(),
            Error("coinbase payload version/height")
        );
        let mut empty_script = Parts {
            script: Vec::new(),
            ..split(&chain.coinbase.transaction)
        };
        assert_eq!(
            rejected(&coinbase(&empty_script, 3, 1, &parts.payload)),
            Error("coinbase input")
        );
        empty_script.script = vec![0; 101];
        assert_eq!(
            rejected(&coinbase(&empty_script, 3, 1, &parts.payload)),
            Error("coinbase input")
        );
    }

    #[test]
    fn should_accept_every_commitment_transaction_shape_consensus_allows() {
        let chain = chain();
        let link = &chain.links[0];
        let height = link.certificate.height - link.ancestors.len() as u32;
        let tx = &link.mining.transaction;
        let expected = mined_commitment(tx, height).unwrap();
        // Core mines commitments with no inputs or outputs.
        assert_eq!(tx[4..6], [0, 0]);
        let payload = {
            let mut r = Reader::new(&tx[10..]);
            let n = r.compact(100_000).unwrap();
            r.take(n).unwrap().to_vec()
        };

        let build = |version: u16, kind: u16| {
            let mut tx = Vec::new();
            tx.extend(version.to_le_bytes());
            tx.extend(kind.to_le_bytes());
            compact(&mut tx, 1);
            tx.extend([0xab; 32]);
            tx.extend(1u32.to_le_bytes());
            compact(&mut tx, 3);
            tx.extend([0x51, 0x52, 0x53]);
            tx.extend(0xffff_fffeu32.to_le_bytes());
            compact(&mut tx, 2);
            for _ in 0..2 {
                tx.extend(1_000u64.to_le_bytes());
                compact(&mut tx, 1);
                tx.push(0x6a);
            }
            tx.extend(7u32.to_le_bytes());
            compact(&mut tx, payload.len());
            tx.extend(&payload);
            tx
        };
        // Inputs, outputs and lock time are free; any special version works.
        assert_eq!(mined_commitment(&build(4, 6), height), Ok(expected));
        assert_eq!(
            mined_commitment(&build(2, 6), height),
            Err(Error("quorum transaction envelope"))
        );
        assert_eq!(
            mined_commitment(&build(3, 5), height),
            Err(Error("quorum transaction envelope"))
        );
        assert_eq!(
            mined_commitment(&build(3, 6), height + 1),
            Err(Error("quorum payload height/version"))
        );
    }

    #[test]
    fn should_reject_noncanonical_commitments() {
        let chain = chain();
        let seed = &chain.seed;
        assert!(parse_commitment(seed).is_ok());
        let mut trailing = seed.clone();
        trailing.push(0);
        assert_eq!(parse_commitment(&trailing), Err(Error("trailing bytes")));
        // The quorum signature is the second-to-last 96 bytes.
        let mut bad_signature = seed.clone();
        let at = bad_signature.len() - 192;
        bad_signature[at..at + 96].fill(0xff);
        assert_eq!(
            parse_commitment(&bad_signature),
            Err(Error("BLS signature encoding"))
        );
        let mut null_signature = seed.clone();
        null_signature[at..at + 96].fill(0);
        assert!(parse_commitment(&null_signature).is_ok());
        let mut wrong_version = seed.clone();
        wrong_version[0] = 2;
        assert_eq!(
            parse_commitment(&wrong_version),
            Err(Error("unsupported commitment version"))
        );
    }
}
