use dpp::dashcore_rpc::json::QuorumType;
use tenderdash_abci::proto;

#[allow(dead_code)]
pub trait CommitAccessorsV0 {
    fn inner(&self) -> &proto::types::Commit;

    fn chain_id(&self) -> &String;

    fn quorum_type(&self) -> QuorumType;
}
