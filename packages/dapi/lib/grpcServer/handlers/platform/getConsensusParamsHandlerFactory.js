const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');

const {
  server: {
    error: {
      InternalGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetConsensusParamsResponse,
    ConsensusParamsBlock,
    ConsensusParamsEvidence,
  },
} = require('@dashevo/dapi-grpc');

const FailedPreconditionGrpcError = require('@dashevo/grpc-common/lib/server/error/FailedPreconditionGrpcError');
const RPCError = require('../../../rpcServer/RPCError');

/**
 *
 * @param {getConsensusParams} getConsensusParams
 * @returns {getConsensusParamsHandler}
 */
function getConsensusParamsHandlerFactory(getConsensusParams) {
  /**
   * @typedef getConsensusParamsHandler
   * @param {Object} call
   * @returns {Promise<>}
   */
  async function getConsensusParamsHandler(call) {
    const { request } = call;

    const prove = request.getProve();

    if (prove) {
      throw new InvalidArgumentGrpcError('Prove is not implemented yet');
    }

    // If height is not set - gRPC returns 0
    // in this case we use undefined
    const height = request.getHeight() || undefined;

    let consensusParams;

    try {
      consensusParams = await getConsensusParams(height);
    } catch (e) {
      if (e instanceof RPCError) {
        if (e.code === 32603) {
          throw new FailedPreconditionGrpcError(`Invalid height: ${e.data}`);
        }

        throw new InternalGrpcError(e);
      }

      throw e;
    }

    const response = new GetConsensusParamsResponse();

    const block = new ConsensusParamsBlock();
    block.setMaxBytes(consensusParams.block.max_bytes);
    block.setMaxGas(consensusParams.block.max_gas);
    block.setTimeIotaMs(consensusParams.block.time_iota_ms);

    response.setBlock(block);

    const evidence = new ConsensusParamsEvidence();
    evidence.setMaxAgeNumBlocks(consensusParams.evidence.max_age_num_blocks);
    evidence.setMaxAgeDuration(consensusParams.evidence.max_age_duration);
    evidence.setMaxBytes(consensusParams.evidence.max_bytes);

    response.setEvidence(evidence);

    return response;
  }

  return getConsensusParamsHandler;
}

module.exports = getConsensusParamsHandlerFactory;
