const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      AlreadyExistsGrpcError,
      ResourceExhaustedGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    BroadcastStateTransitionResponse,
  },
} = require('@dashevo/dapi-grpc');
const logger = require('../../../logger');

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

    let response;

    try {
      response = await rpcClient.request('broadcast_tx_sync', { tx });
    } catch (e) {
      logger.error(e, 'Failed broadcasting state transition');

      throw e;
    }

    const { result, error: jsonRpcError } = response;

    if (jsonRpcError) {
      if (typeof jsonRpcError.data === 'string') {
        if (jsonRpcError.data === 'tx already exists in cache') {
          throw new AlreadyExistsGrpcError('state transition already in chain');
        }

        if (jsonRpcError.data.startsWith('Tx too large.')) {
          const message = jsonRpcError.data.replace('Tx too large. ', '');
          throw new InvalidArgumentGrpcError(`state transition is too large. ${message}`);
        }

        if (jsonRpcError.data.startsWith('mempool is full')) {
          throw new ResourceExhaustedGrpcError(jsonRpcError.data);
        }
      }

      const error = new Error();
      Object.assign(error, jsonRpcError);

      logger.error(error, 'Unexpected JSON RPC error during broadcasting state transition');

      throw error;
    }

    if (result.code !== 0) {
      throw await createGrpcErrorFromDriveResponse(result.code, result.info);
    }

    return new BroadcastStateTransitionResponse();
  }

  return broadcastStateTransitionHandler;
}

module.exports = broadcastStateTransitionHandlerFactory;
