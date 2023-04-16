use dashcore::QuorumHash;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
/// Quorum information
#[derive(Clone)]
pub struct Quorum {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// The list of masternodes
    pub validator_set: Vec<MasternodeListItem>,
    /// The quorum public key
    pub public_key: BlsPublicKey,
}
