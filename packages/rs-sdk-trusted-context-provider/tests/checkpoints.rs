//! The embedded checkpoints are what the chain says they are.
//!
//! `tests/data/checkpoint-evidence-<network>.json` holds the checkpoint
//! block's raw 80-byte header, its raw coinbase transaction and the coinbase's
//! Merkle branch (sibling hashes in wire order). The test recomputes the block
//! hash from the header, the header's Merkle root from the coinbase, and the
//! masternode and quorum roots from the coinbase payload, so a checkpoint file
//! cannot be edited without the evidence also being forged.
use dash_core_proof::{coinbase_roots, header_hash, sha256d, MerklePath};
use dpp::dashcore::Network;
use rs_sdk_trusted_context_provider::VerifiedHttpContextProvider;
use serde_json::Value;

fn bytes(value: &Value) -> Vec<u8> {
    hex::decode(value.as_str().expect("a hex string")).expect("hex")
}

#[test]
fn should_embed_checkpoints_that_match_their_block_header_and_coinbase() {
    for (network, evidence) in [
        (
            Network::Mainnet,
            include_str!("data/checkpoint-evidence-mainnet.json"),
        ),
        (
            Network::Testnet,
            include_str!("data/checkpoint-evidence-testnet.json"),
        ),
    ] {
        let checkpoint =
            VerifiedHttpContextProvider::embedded_checkpoint(network).expect("a valid checkpoint");
        let evidence: Value = serde_json::from_str(evidence).expect("evidence JSON");

        let header: [u8; 80] = bytes(&evidence["header"])
            .try_into()
            .expect("an 80-byte header");
        assert_eq!(header_hash(&header), checkpoint.block_hash, "{network:?}");

        let coinbase = bytes(&evidence["coinbase"]);
        let path = &evidence["coinbase_path"];
        let siblings = path["siblings"]
            .as_array()
            .expect("siblings")
            .iter()
            .map(|sibling| bytes(sibling).try_into().expect("a 32-byte hash"))
            .collect();
        let path = MerklePath {
            index: path["index"].as_u64().expect("an index") as u32,
            count: path["count"].as_u64().expect("a count") as u32,
            siblings,
        };
        assert_eq!(path.index, 0, "the coinbase is the first transaction");
        let merkle_root: [u8; 32] = header[36..68].try_into().expect("32 bytes");
        path.verify(sha256d(&coinbase), &merkle_root)
            .expect("the coinbase is in the checkpoint block");

        assert_eq!(
            coinbase_roots(&coinbase, checkpoint.height).expect("a coinbase payload"),
            (checkpoint.masternode_root, checkpoint.quorum_root),
            "{network:?}"
        );
    }
}
