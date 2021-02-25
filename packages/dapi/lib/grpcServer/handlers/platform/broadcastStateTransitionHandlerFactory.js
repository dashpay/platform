const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      FailedPreconditionGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BroadcastStateTransitionResponse,
  },
} = require('@dashevo/dapi-grpc');

const AbciResponseError = require('../../../errors/AbciResponseError');

/**
 * @param {jaysonClient} rpcClient
 * @param {handleAbciResponseError} handleAbciResponseError
 *
 * @returns {broadcastStateTransitionHandler}
 */
function broadcastStateTransitionHandlerFactory(rpcClient, handleAbciResponseError) {
  /**
   * @typedef broadcastStateTransitionHandler
   *
   * @param {Object} call
   *
   * @return {Promise<BroadcastStateTransitionResponse>}
   */
  async function broadcastStateTransitionHandler(call) {
    const { request } = call;
    const stByteArray = request.getStateTransition();

    if (!stByteArray) {
      throw new InvalidArgumentGrpcError('State Transition is not specified');
    }

    const tx = Buffer.from(stByteArray).toString('base64');

    const { result, error: jsonRpcError } = await rpcClient.request('broadcast_tx_sync', { tx });

    if (jsonRpcError) {
      if (jsonRpcError.data === 'tx already exists in cache') {
        throw new FailedPreconditionGrpcError(jsonRpcError.data, jsonRpcError);
      }

      const error = new Error();
      Object.assign(error, jsonRpcError);

      throw error;
    }

    if (result.code !== undefined && result.code !== 0) {
      const { error: abciError } = JSON.parse(result.log);

      handleAbciResponseError(
        new AbciResponseError(result.code, abciError),
      );
    }

    return new BroadcastStateTransitionResponse();
  }

  return broadcastStateTransitionHandler;
}

module.exports = broadcastStateTransitionHandlerFactory;
