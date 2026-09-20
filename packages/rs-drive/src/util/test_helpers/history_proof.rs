//! Legacy envelope fixtures for document-history format validation.
use dpp::version::PlatformVersion;
use grovedb::operations::proof::{
    GroveDBProof, GroveDBProofV0, LayerProof, MerkOnlyLayerProof, ProofBytes, ProveOptions,
};
use grovedb::Element;
use grovedb_query::proofs::{encode_into, Decoder, Node, Op};

/// Converts a proof to the legacy envelope with an optional terminal count override.
/// The legacy node retains the committed value hash independently of its bytes.
pub fn downgrade_history_count(
    proof: &[u8],
    count: Option<u64>,
    version: &PlatformVersion,
) -> Vec<u8> {
    fn downgrade(
        layer: LayerProof,
        count: Option<u64>,
        version: &PlatformVersion,
        changed: &mut usize,
    ) -> MerkOnlyLayerProof {
        let ProofBytes::Merk(bytes) = layer.merk_proof else {
            panic!("expected Merk layer")
        };
        let ops: Vec<_> = Decoder::new(&bytes)
            .map(|op| {
                let op = op.unwrap();
                let (node, inverted) = match op {
                    Op::Push(node) => (node, false),
                    Op::PushInverted(node) => (node, true),
                    other => return other,
                };
                let node = match node {
                    Node::KVValueHashFeatureTypeWithChildHash(key, value, hash, feature, _) => {
                        let mut element =
                            Element::deserialize(&value, &version.drive.grove_version).unwrap();
                        if let (
                            Some(count),
                            Element::ProvableCountTree(_, ref mut stored_count, _),
                        ) = (count, &mut element)
                        {
                            *stored_count = count;
                            *changed += 1;
                        }
                        Node::KVValueHashFeatureType(
                            key,
                            element.serialize(&version.drive.grove_version).unwrap(),
                            hash,
                            feature,
                        )
                    }
                    other => other,
                };
                if inverted {
                    Op::PushInverted(node)
                } else {
                    Op::Push(node)
                }
            })
            .collect();
        let mut bytes = vec![];
        encode_into(ops.iter(), &mut bytes);
        MerkOnlyLayerProof {
            merk_proof: bytes,
            lower_layers: layer
                .lower_layers
                .into_iter()
                .map(|(key, layer)| (key, downgrade(layer, count, version, changed)))
                .collect(),
        }
    }
    let config = bincode::config::standard().with_big_endian();
    let (proof, _): (GroveDBProof, _) = bincode::decode_from_slice(proof, config).unwrap();
    let GroveDBProof::V1(proof) = proof else {
        panic!("expected V1 proof")
    };
    let mut changed = 0;
    let root_layer = downgrade(proof.root_layer, count, version, &mut changed);
    assert_eq!(
        changed,
        usize::from(count.is_some()),
        "must override exactly the terminal history tree"
    );
    bincode::encode_to_vec(
        GroveDBProof::V0(GroveDBProofV0 {
            root_layer,
            prove_options: ProveOptions::default(),
        }),
        config,
    )
    .unwrap()
}
