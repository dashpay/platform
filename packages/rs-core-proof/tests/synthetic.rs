//! Chains built from scratch with known BLS keys, to reach rejections no
//! valid chain data can: a repeated signer, a non-ChainLock quorum, a v4
//! coinbase from end to end, and each envelope and commitment limit.
use blst::min_pk::SecretKey;
use dash_core_proof::{
    bootstrap, coinbase_roots, header_hash, parse_commitment, sha256d, Certificate, Hash, Link,
    MerklePath, ProofNetwork, ProofTransaction, QuorumProofChain, State,
};

const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
/// Testnet's ChainLock quorum type, LLMQ_50_60.
const CHAINLOCK: u8 = 1;
const CHECKPOINT_HEIGHT: u32 = 1_000_000;

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

fn node(left: &Hash, right: &Hash) -> Hash {
    let mut pair = [0; 64];
    pair[..32].copy_from_slice(left);
    pair[32..].copy_from_slice(right);
    sha256d(&pair)
}

/// A commitment's parts, so tests can break one rule at a time.
#[derive(Clone)]
struct CommitmentSpec {
    version: u16,
    quorum_type: u8,
    quorum_hash: Hash,
    quorum_index: Option<u16>,
    size: usize,
    signers: usize,
    padding: bool,
    public_key: [u8; 48],
}

impl CommitmentSpec {
    fn new(quorum_type: u8, quorum_hash: Hash, public_key: [u8; 48]) -> Self {
        let (size, threshold, rotated) = match quorum_type {
            1 => (50, 30, false),
            4 => (100, 67, false),
            5 => (60, 45, true),
            6 => (25, 17, false),
            _ => unreachable!("test helper covers the types it uses"),
        };
        Self {
            version: if rotated { 4 } else { 3 },
            quorum_type,
            quorum_hash,
            quorum_index: rotated.then_some(0),
            size,
            signers: threshold,
            padding: false,
            public_key,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(self.version.to_le_bytes());
        out.push(self.quorum_type);
        out.extend(self.quorum_hash);
        if let Some(index) = self.quorum_index {
            out.extend(index.to_le_bytes());
        }
        for _ in 0..2 {
            compact(&mut out, self.size);
            let mut bits = vec![0u8; self.size.div_ceil(8)];
            for i in 0..self.signers {
                bits[i / 8] |= 1 << (i % 8);
            }
            if self.padding {
                *bits.last_mut().unwrap() |= 0x80;
            }
            out.extend(bits);
        }
        out.extend(self.public_key);
        out.extend([7; 32]); // verification vector hash
        out.extend([0; 192]); // null quorum and members' signatures
        out
    }
}

struct Quorum {
    secret: SecretKey,
    quorum_type: u8,
    quorum_hash: Hash,
}

impl Quorum {
    fn new(seed: u8, quorum_type: u8) -> Self {
        Self {
            secret: SecretKey::key_gen(&[seed; 32], &[]).unwrap(),
            quorum_type,
            quorum_hash: [seed; 32],
        }
    }

    fn commitment(&self) -> Vec<u8> {
        let public_key = self.secret.sk_to_pk().compress();
        CommitmentSpec::new(self.quorum_type, self.quorum_hash, public_key).encode()
    }

    /// A ChainLock for `header` at `height`, computed independently of the crate.
    fn certify(&self, height: u32, header: [u8; 80]) -> Certificate {
        let mut request = b"\x05clsig".to_vec();
        request.extend(height.to_le_bytes());
        let mut message = vec![self.quorum_type];
        message.extend(self.quorum_hash);
        message.extend(sha256d(&request));
        message.extend(header_hash(&header));
        let signature = self.secret.sign(&sha256d(&message), DST, &[]).compress();
        Certificate {
            height,
            header,
            signature,
        }
    }
}

fn header(prev: Hash, merkle_root: Hash) -> [u8; 80] {
    let mut header = [0; 80];
    header[..4].copy_from_slice(&4u32.to_le_bytes());
    header[4..36].copy_from_slice(&prev);
    header[36..68].copy_from_slice(&merkle_root);
    header
}

fn mining_transaction(height: u32, commitment: &[u8]) -> Vec<u8> {
    let mut payload = 1u16.to_le_bytes().to_vec();
    payload.extend(height.to_le_bytes());
    payload.extend(commitment);
    let mut tx = Vec::new();
    tx.extend(3u16.to_le_bytes());
    tx.extend(6u16.to_le_bytes());
    tx.extend([0, 0]); // no inputs or outputs
    tx.extend(0u32.to_le_bytes());
    compact(&mut tx, payload.len());
    tx.extend(payload);
    tx
}

fn coinbase(version: u16, height: u32, roots: (Hash, Hash), chainlock_diff: u8) -> Vec<u8> {
    let mut payload = version.to_le_bytes().to_vec();
    payload.extend(height.to_le_bytes());
    payload.extend(roots.0);
    payload.extend(roots.1);
    payload.push(chainlock_diff);
    payload.extend([0; 96]); // null best ChainLock signature
    payload.extend(0u64.to_le_bytes());
    if version >= 4 {
        payload.extend([9; 32]); // merkleRootAssetUnlocks
    }
    let mut tx = Vec::new();
    tx.extend(3u16.to_le_bytes());
    tx.extend(5u16.to_le_bytes());
    compact(&mut tx, 1);
    tx.extend([0; 32]);
    tx.extend(u32::MAX.to_le_bytes());
    compact(&mut tx, 2);
    tx.extend([1, 2]);
    tx.extend(u32::MAX.to_le_bytes());
    compact(&mut tx, 1);
    tx.extend(0u64.to_le_bytes());
    compact(&mut tx, 0);
    tx.extend(0u32.to_le_bytes());
    compact(&mut tx, payload.len());
    tx.extend(payload);
    tx
}

/// A link certifying the block that mines `next`, a block with a dummy
/// coinbase at index 0 and the commitment at index 1.
fn link(signer: &Quorum, next_commitment: &[u8], height: u32) -> Link {
    let dummy = [0x42; 100];
    let mining = mining_transaction(height, next_commitment);
    let root = node(&sha256d(&dummy), &sha256d(&mining));
    Link {
        certificate: signer.certify(height, header([height as u8; 32], root)),
        mining: ProofTransaction {
            transaction: mining,
            path: MerklePath {
                index: 1,
                count: 2,
                siblings: vec![sha256d(&dummy)],
            },
        },
        ancestors: Vec::new(),
    }
}

struct Built {
    checkpoint: State,
    chain: QuorumProofChain,
    record: Vec<u8>,
}

/// A testnet chain: the checkpoint's ChainLock quorum `seed`, one link per
/// further quorum, and a target whose quorum root is the single `record`.
fn build(seed: &Quorum, next: &[&Quorum], coinbase_version: u16) -> Built {
    let seed_commitment = seed.commitment();
    let checkpoint = State {
        network: ProofNetwork::Testnet,
        height: CHECKPOINT_HEIGHT,
        block_hash: [1; 32],
        masternode_root: [2; 32],
        quorum_root: sha256d(&seed_commitment),
    };
    let mut signer = seed;
    let mut links = Vec::new();
    let mut height = CHECKPOINT_HEIGHT;
    for quorum in next {
        height += 24;
        links.push(link(signer, &quorum.commitment(), height));
        signer = quorum;
    }
    let record = Quorum::new(0xee, 6).commitment();
    let target_height = height + 10;
    let cb = coinbase(
        coinbase_version,
        target_height,
        ([3; 32], sha256d(&record)),
        0,
    );
    let target = signer.certify(target_height, header([0xaa; 32], sha256d(&cb)));
    let chain = QuorumProofChain {
        anchor: checkpoint.clone(),
        seed: seed_commitment,
        seed_path: MerklePath {
            index: 0,
            count: 1,
            siblings: Vec::new(),
        },
        links,
        target,
        coinbase: ProofTransaction {
            transaction: cb,
            path: MerklePath {
                index: 0,
                count: 1,
                siblings: Vec::new(),
            },
        },
    };
    Built {
        checkpoint,
        chain,
        record,
    }
}

/// A bootstrap envelope around `chain` with raw `(kind, leaf, path)` records.
fn envelope(chain: &QuorumProofChain, count: u8, records: &[(u8, &[u8])]) -> Vec<u8> {
    let proof = chain.encode().unwrap();
    let mut out = (proof.len() as u32).to_le_bytes().to_vec();
    out.extend(proof);
    out.push(count);
    for (kind, leaf) in records {
        out.push(*kind);
        out.extend((leaf.len() as u32).to_le_bytes());
        out.extend(*leaf);
        out.extend(0u32.to_le_bytes()); // index
        out.extend(1u32.to_le_bytes()); // count
        out.push(0); // no siblings
    }
    out
}

#[test]
fn should_verify_chains_ending_in_v3_and_v4_coinbases() {
    let (a, b, c) = (
        Quorum::new(1, CHAINLOCK),
        Quorum::new(2, CHAINLOCK),
        Quorum::new(3, CHAINLOCK),
    );
    for (next, version) in [(vec![], 3), (vec![&b, &c], 3), (vec![&b, &c], 4)] {
        let built = build(&a, &next, version);
        let state = built.chain.verify(&built.checkpoint).unwrap();
        assert_eq!(state.quorum_root, sha256d(&built.record));
        assert_eq!(state.height, built.chain.target.height);

        let bytes = envelope(&built.chain, 1, &[(0, &built.record)]);
        let verified = bootstrap::verify(&bytes, &built.checkpoint, state.height).unwrap();
        assert_eq!(verified.state(), &state);
        assert_eq!(
            parse_commitment(verified.records()[0].leaf)
                .unwrap()
                .quorum_type,
            6
        );
    }
}

#[test]
fn should_reject_a_link_that_mines_its_own_signer() {
    let (a, b) = (Quorum::new(1, CHAINLOCK), Quorum::new(2, CHAINLOCK));
    let mut built = build(&a, &[&b], 3);
    built.chain.links[0] = link(&a, &a.commitment(), CHECKPOINT_HEIGHT + 24);
    assert_eq!(
        built.chain.verify(&built.checkpoint).unwrap_err().0,
        "redundant signer"
    );
}

#[test]
fn should_reject_signers_that_are_not_chainlock_quorums() {
    let platform = Quorum::new(9, 6);
    let built = build(&platform, &[], 3);
    assert_eq!(
        built.chain.verify(&built.checkpoint).unwrap_err().0,
        "unexpected quorum type"
    );

    let a = Quorum::new(1, CHAINLOCK);
    let built = build(&a, &[&platform], 3);
    assert_eq!(
        built.chain.verify(&built.checkpoint).unwrap_err().0,
        "unexpected quorum type"
    );

    // Mainnet ChainLocks are LLMQ_400_60, so a testnet type is wrong there.
    let mut built = build(&a, &[], 3);
    built.checkpoint.network = ProofNetwork::Mainnet;
    built.checkpoint.height = 2_000_000;
    built.chain.anchor = built.checkpoint.clone();
    assert_eq!(
        built.chain.verify(&built.checkpoint).unwrap_err().0,
        "unexpected quorum type"
    );
}

#[test]
fn should_reject_envelope_record_limits() {
    let a = Quorum::new(1, CHAINLOCK);
    let built = build(&a, &[], 3);
    let ok = |count, records: &[(u8, &[u8])]| {
        bootstrap::verify(
            &envelope(&built.chain, count, records),
            &built.checkpoint,
            0,
        )
        .map(|_| ())
        .map_err(|e| e.0)
    };
    assert_eq!(ok(1, &[(0, &built.record)]), Ok(()));
    assert_eq!(ok(0, &[]), Err("projection count"));
    assert_eq!(ok(17, &[]), Err("projection count"));
    assert_eq!(ok(1, &[(2, &built.record)]), Err("record kind"));
    assert_eq!(ok(1, &[(0, &[])]), Err("record leaf size"));
    assert_eq!(ok(1, &[(0, &[5; 64])]), Err("record leaf size"));
    assert_eq!(ok(1, &[(0, &[5; 4097])]), Err("blob limit"));
    // The record must open under the right root.
    assert_eq!(ok(1, &[(1, &built.record)]), Err("Merkle root/path"));
    // A stale target is refused before any signature is checked.
    let bytes = envelope(&built.chain, 1, &[(0, &built.record)]);
    let stale = bootstrap::verify(&bytes, &built.checkpoint, built.chain.target.height + 1);
    assert_eq!(stale.unwrap_err().0, "stale target");
}

#[test]
fn should_reject_coinbase_chainlock_diff_reaching_the_block() {
    let roots = ([3; 32], [4; 32]);
    assert_eq!(
        coinbase_roots(&coinbase(3, 200, roots, 199), 200),
        Ok(roots)
    );
    assert_eq!(
        coinbase_roots(&coinbase(3, 200, roots, 200), 200)
            .unwrap_err()
            .0,
        "coinbase ChainLock height"
    );
    assert_eq!(
        coinbase_roots(&coinbase(3, 200, ([3; 32], [0; 32]), 0), 200)
            .unwrap_err()
            .0,
        "coinbase quorum root"
    );
    assert_eq!(
        coinbase_roots(&coinbase(2, 200, roots, 0), 200)
            .unwrap_err()
            .0,
        "coinbase payload version/height"
    );
}

#[test]
fn should_reject_commitments_core_rejects() {
    let key = SecretKey::key_gen(&[1; 32], &[])
        .unwrap()
        .sk_to_pk()
        .compress();
    let rejects = |spec: CommitmentSpec| parse_commitment(&spec.encode()).unwrap_err().0;
    let platform = CommitmentSpec::new(6, [5; 32], key);
    let rotated = CommitmentSpec::new(5, [5; 32], key);
    assert!(parse_commitment(&platform.encode()).is_ok());
    assert!(parse_commitment(&rotated.encode()).is_ok());

    assert_eq!(
        rejects(CommitmentSpec {
            signers: platform.signers - 1,
            ..platform.clone()
        }),
        "null/undersized quorum"
    );
    assert_eq!(
        rejects(CommitmentSpec {
            padding: true,
            ..platform.clone()
        }),
        "bitset padding"
    );
    assert_eq!(
        rejects(CommitmentSpec {
            size: 24,
            signers: 17,
            ..platform.clone()
        }),
        "quorum bit count"
    );
    assert_eq!(
        rejects(CommitmentSpec {
            quorum_hash: [0; 32],
            ..platform.clone()
        }),
        "null quorum"
    );
    assert_eq!(
        rejects(CommitmentSpec {
            version: 4,
            quorum_index: Some(0),
            ..platform.clone()
        }),
        "commitment rotation version"
    );
    assert_eq!(
        rejects(CommitmentSpec {
            version: 3,
            quorum_index: None,
            ..rotated.clone()
        }),
        "commitment rotation version"
    );
    for index in [32, 0x8000] {
        assert_eq!(
            rejects(CommitmentSpec {
                quorum_index: Some(index),
                ..rotated.clone()
            }),
            "quorum index"
        );
    }
}
