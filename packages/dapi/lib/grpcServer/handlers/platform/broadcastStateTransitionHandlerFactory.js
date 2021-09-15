const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      AlreadyExistsGrpcError,
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
 * @param {createGrpcErrorFromDriveResponse} createGrpcErrorFromDriveResponse
 *
 * @returns {broadcastStateTransitionHandler}
 */
function broadcastStateTransitionHandlerFactory(rpcClient, createGrpcErrorFromDriveResponse) {
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
        throw new AlreadyExistsGrpcError('State transition already in chain', jsonRpcError);
      }

      const error = new Error();
      Object.assign(error, jsonRpcError);

      throw error;
    }

    if (result.code !== 0) {
      throw createGrpcErrorFromDriveResponse(result.code, result.info);
    }

    return new BroadcastStateTransitionResponse();
  }

  return broadcastStateTransitionHandler;
}

module.exports = broadcastStateTransitionHandlerFactory;
