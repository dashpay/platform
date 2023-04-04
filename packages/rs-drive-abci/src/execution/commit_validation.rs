use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::bls_signatures;
use dpp::bls_signatures::Serialize;
use dpp::validation::SimpleValidationResult;
use drive::grovedb::Transaction;
use tenderdash_abci::proto::abci::CommitInfo;
use tenderdash_abci::proto::types::BlockId;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Validates a commit received in finalize block
    /// An explanation can be found here
    /// https://github.com/dashpay/tenderdash/blob/v0.12-dev/spec/consensus/signing.md#block-signature-verification-on-light-client
    ///
    /// Verification algorithm can be described as follows:
    ///
    /// Build StateID message and encode it using Protobuf encoding.
    /// Calculate checksum (SHA256) of encoded StateID.
    /// Retrieve or calculate SHA256 checksum of CanonicalBlockID
    /// Build CanonicalVote message and encode it using Protobuf.
    /// Calculate SHA256 checksum of encoded CanonicalVote.
    /// Verify that block signature matches calculated checksum.
    pub(crate) fn validate_commit(
        &self,
        commit: CommitInfo,
        block_id: BlockId,
    ) -> Result<SimpleValidationResult<AbciError>, Error> {
        let signature = commit.block_signature;
        // We could have received a fake commit, so signature validation needs to be returned if error as a simple validation result
        let signature = match bls_signatures::Signature::from_bytes(signature.as_slice()) {
            Ok(signature) => signature,
            Err(e) => return Ok(SimpleValidationResult::new_with_error(e.into())),
        };
        let public_key = self
            .core_rpc
            .get_quorum_info(
                self.config.quorum_type.clone(),
                &QuorumHash {
                    0: commit.quorum_hash,
                },
            )?
            .quorum_public_key;
        let public_key = match bls_signatures::PublicKey::from_bytes(signature.as_slice()) {
            Ok(public_key) => public_key,
            Err(e) => return Ok(SimpleValidationResult::new_with_error(e.into())),
        };

        // todo: public_key.verify(signature, )

        Ok(SimpleValidationResult::default())
    }
}
