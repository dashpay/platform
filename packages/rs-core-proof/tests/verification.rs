//! Real proofs from Dash Core 23.1.8 nodes' `getquorumproofchain`.
//!
//! `tests/data/<network>.json` holds the checkpoint (from `getblock`), the RPC's
//! `proof_hex`, `bootstrap_hex` and `target`. Each request opened one Platform
//! quorum (LLMQ_100_67 on mainnet, LLMQ_25_67 on testnet) and two EvoNodes.
//! The mainnet chain has 2 links and the testnet one 4, two of which were
//! mined in blocks below the certified ones (2 and 3 ancestor headers).
//! `testnet-no-links.json` is a proof from a recent checkpoint, signed by the
//! checkpoint's own ChainLock quorum.
use dash_core_proof::bootstrap::{self, RecordKind};
use dash_core_proof::{parse_commitment, rpc_hex, QuorumProofChain, State, MAX_PROOF_HEADERS};

struct Fixture {
    checkpoint: State,
    proof: Vec<u8>,
    bootstrap: Vec<u8>,
    target: State,
}

fn fixture(json: &str) -> Fixture {
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let hex = |key: &str| hex::decode(value[key].as_str().unwrap()).unwrap();
    Fixture {
        checkpoint: serde_json::from_value(value["checkpoint"].clone()).unwrap(),
        proof: hex("proof_hex"),
        bootstrap: hex("bootstrap_hex"),
        target: serde_json::from_value(value["target"].clone()).unwrap(),
    }
}

fn mainnet() -> Fixture {
    fixture(include_str!("data/mainnet.json"))
}

fn testnet() -> Fixture {
    fixture(include_str!("data/testnet.json"))
}

#[test]
fn should_verify_core_proofs_to_the_target_core_reported() {
    for (f, links, platform_quorum_type) in [(mainnet(), 2, 4), (testnet(), 4, 6)] {
        let chain = QuorumProofChain::decode(&f.proof).unwrap();
        assert_eq!(chain.links.len(), links);
        assert_eq!(chain.encode().unwrap(), f.proof, "decoding is canonical");
        assert_eq!(chain.verify(&f.checkpoint).unwrap(), f.target);

        let verified = bootstrap::verify(&f.bootstrap, &f.checkpoint, f.target.height).unwrap();
        assert_eq!(verified.state(), &f.target);
        let kinds: Vec<_> = verified.records().iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [
                RecordKind::Quorum,
                RecordKind::Masternode,
                RecordKind::Masternode
            ]
        );
        let quorum = parse_commitment(verified.records()[0].leaf).unwrap();
        assert_eq!(quorum.quorum_type, platform_quorum_type);
    }
}

#[test]
fn should_open_the_requested_mainnet_quorum() {
    let f = mainnet();
    let verified = bootstrap::verify(&f.bootstrap, &f.checkpoint, 0).unwrap();
    let quorum = parse_commitment(verified.records()[0].leaf).unwrap();
    assert_eq!(
        rpc_hex(&quorum.quorum_hash),
        "000000000000001e3234155b7645fdd5af10c59bd1507ee6d75ded0ae4eab979"
    );
}

#[test]
fn should_reject_a_stale_target_or_another_checkpoint() {
    let f = testnet();
    let stale = bootstrap::verify(&f.bootstrap, &f.checkpoint, f.target.height + 1);
    assert_eq!(stale.unwrap_err().0, "stale target");

    let mut other = f.checkpoint.clone();
    other.quorum_root[0] ^= 1;
    assert_eq!(
        bootstrap::verify(&f.bootstrap, &other, 0).unwrap_err().0,
        "untrusted snapshot"
    );
    // A checkpoint on another network never matches either.
    let mainnet = mainnet();
    assert!(bootstrap::verify(&f.bootstrap, &mainnet.checkpoint, 0).is_err());
}

#[test]
fn should_reject_every_truncation() {
    for f in [mainnet(), testnet()] {
        for n in 0..f.proof.len() {
            assert!(
                QuorumProofChain::decode(&f.proof[..n]).is_err(),
                "proof prefix {n}"
            );
        }
        // Past the chain, each prefix costs a full verification; a stride
        // still cuts inside every record field.
        for n in (0..f.bootstrap.len()).step_by(7) {
            assert!(
                bootstrap::verify(&f.bootstrap[..n], &f.checkpoint, 0).is_err(),
                "bootstrap prefix {n}"
            );
        }
    }
}

#[test]
fn should_reject_byte_flips_across_the_envelope() {
    let f = testnet();
    // Every 61st byte reaches each section of the envelope; each flip is checked
    // through full verification.
    for i in (0..f.bootstrap.len()).step_by(61) {
        let mut corrupt = f.bootstrap.clone();
        corrupt[i] ^= 0x01;
        assert!(
            bootstrap::verify(&corrupt, &f.checkpoint, 0).is_err(),
            "flip at {i}"
        );
    }
}

#[test]
fn should_reject_tampered_links() {
    let f = testnet();
    let chain = QuorumProofChain::decode(&f.proof).unwrap();
    let with_ancestors = chain
        .links
        .iter()
        .position(|link| link.ancestors.len() > 1)
        .unwrap();
    let rejects = |mutate: &dyn Fn(&mut QuorumProofChain)| {
        let mut bad = chain.clone();
        mutate(&mut bad);
        bad.verify(&f.checkpoint).unwrap_err().0
    };

    assert_eq!(rejects(&|c| c.target.signature[5] ^= 1), "BLS signature");
    assert_eq!(
        rejects(&|c| c.links[with_ancestors].ancestors.clear()),
        "Merkle root/path"
    );
    assert_eq!(
        rejects(&|c| c.links[with_ancestors].ancestors[0][0] ^= 1),
        "ancestor continuity"
    );
    assert_eq!(
        rejects(&|c| {
            c.links[with_ancestors].ancestors.remove(0);
        }),
        "Merkle root/path"
    );
    assert_eq!(
        rejects(&|c| {
            c.links.remove(0);
        }),
        "BLS verification"
    );
    assert_eq!(
        rejects(&|c| c.target.height = c.links.last().unwrap().certificate.height),
        "non-increasing certificate"
    );
    // The seed's verification vector hash: still a well-formed commitment,
    // but no longer the one under the checkpoint's quorum root.
    assert_eq!(
        rejects(&|c| {
            let at = c.seed.len() - 224;
            c.seed[at] ^= 1;
        }),
        "Merkle root/path"
    );
    assert_eq!(
        rejects(&|c| c.coinbase.path.index = 1),
        "coinbase inclusion"
    );
    assert_eq!(
        rejects(&|c| c.links[0].mining.path.index = 0),
        "mining inclusion"
    );
    // Replaying a link is caught by the certificate heights.
    assert_eq!(
        rejects(&|c| {
            let first = c.links[0].clone();
            c.links.insert(0, first);
        }),
        "non-increasing certificate"
    );
}

#[test]
fn should_reject_chains_over_the_header_budget() {
    let f = testnet();
    let mut chain = QuorumProofChain::decode(&f.proof).unwrap();
    let header = chain.links[0].certificate.header;
    chain.links[0].ancestors = vec![header; MAX_PROOF_HEADERS + 1];
    assert!(chain.encode().is_err());
    assert_eq!(
        chain.verify(&f.checkpoint).unwrap_err().0,
        "ancestor budget/height"
    );
}

#[test]
fn should_verify_a_proof_signed_by_the_checkpoint_quorum_itself() {
    let f = fixture(include_str!("data/testnet-no-links.json"));
    let chain = QuorumProofChain::decode(&f.proof).unwrap();
    assert!(chain.links.is_empty());
    assert_eq!(chain.verify(&f.checkpoint).unwrap(), f.target);
    let verified = bootstrap::verify(&f.bootstrap, &f.checkpoint, f.target.height).unwrap();
    assert_eq!(verified.state(), &f.target);
}
