use crate::platform_types::commit::v0::accessors::CommitAccessorsV0;
use crate::platform_types::commit::Commit;
use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;

impl CommitAccessorsV0 for Commit {
    fn inner(&self) -> &tenderdash_abci::proto::types::Commit {
        match self {
            Commit::V0(v0) => &v0.inner,
        }
    }

    fn chain_id(&self) -> &String {
        match self {
            Commit::V0(v0) => &v0.chain_id,
        }
    }

    fn quorum_type(&self) -> QuorumType {
        match self {
            Commit::V0(v0) => v0.quorum_type,
        }
    }
}
