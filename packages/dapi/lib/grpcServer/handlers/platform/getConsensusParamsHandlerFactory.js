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
  },
} = require('@dashevo/dapi-grpc');

const FailedPreconditionGrpcError = require('@dashevo/grpc-common/lib/server/error/FailedPreconditionGrpcError');
const UnavailableGrpcError = require('@dashevo/grpc-common/lib/server/error/UnavailableGrpcError');
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

    const prove = request.getV0().getProve();

    if (prove) {
      throw new InvalidArgumentGrpcError('Prove is not implemented yet');
    }

    // If height is not set - gRPC returns 0
    // in this case we use undefined
    const height = request.getV0().getHeight() || undefined;

    let consensusParams;

    try {
      consensusParams = await getConsensusParams(height);
    } catch (e) {
      if (e.message === 'socket hang up') {
        throw new UnavailableGrpcError('Tenderdash is not available');
      }

      if (e instanceof RPCError) {
        if (e.code === -32603) {
          throw new FailedPreconditionGrpcError(`Invalid height: ${e.data}`);
        }

        throw new InternalGrpcError(e);
      }

      throw e;
    }

    const response = new GetConsensusParamsResponse();
    const {
      ConsensusParamsBlock,
      GetConsensusParamsResponseV0,
      ConsensusParamsEvidence,
    } = GetConsensusParamsResponse;

    const block = new ConsensusParamsBlock();
    block.setMaxBytes(consensusParams.block.max_bytes);
    block.setMaxGas(consensusParams.block.max_gas);
    block.setTimeIotaMs(consensusParams.block.time_iota_ms);

    const evidence = new ConsensusParamsEvidence();
    evidence.setMaxAgeNumBlocks(consensusParams.evidence.max_age_num_blocks);
    evidence.setMaxAgeDuration(consensusParams.evidence.max_age_duration);
    evidence.setMaxBytes(consensusParams.evidence.max_bytes);

    response.setV0(
      new GetConsensusParamsResponseV0()
        .setBlock(block)
        .setEvidence(evidence),
    );

    return response;
  }

  return getConsensusParamsHandler;
}

module.exports = getConsensusParamsHandlerFactory;
