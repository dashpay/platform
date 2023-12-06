mod v0;

use std::collections::BTreeMap;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::dashcore::{ChainLock, QuorumHash};
use dpp::platform_value::Bytes32;
use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use dpp::version::PlatformVersion;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    ///
    pub fn choose_quorum<'a>(&self, llmq_quorum_type: QuorumType, quorums: &'a BTreeMap<QuorumHash, BlsPublicKey>, request_id: &[u8;32], platform_version: &PlatformVersion) -> Result<Option<(&'a QuorumHash, &'a BlsPublicKey)>, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .choose_quorum
        {
            0 => {
                Ok(self.choose_quorum_v0(llmq_quorum_type,  quorums, request_id, platform_version))
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "choose_quorum".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
