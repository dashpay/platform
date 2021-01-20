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

/**
 * @param {jaysonClient} rpcClient
 *
 * @returns {broadcastStateTransitionHandler}
 */
function broadcastStateTransitionHandlerFactory(rpcClient) {
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

    const { error: jsonRpcError } = await rpcClient.request('broadcast_tx_async', { tx });

    if (jsonRpcError) {
      if (jsonRpcError.data === 'tx already exists in cache') {
        throw new FailedPreconditionGrpcError(jsonRpcError.data, jsonRpcError);
      }

      const error = new Error();
      Object.assign(error, jsonRpcError);

      throw error;
    }

    return new BroadcastStateTransitionResponse();
  }

  return broadcastStateTransitionHandler;
}

module.exports = broadcastStateTransitionHandlerFactory;
