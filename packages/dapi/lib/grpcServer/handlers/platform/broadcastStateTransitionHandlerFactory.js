const {
  server: {
    error: {
      InvalidArgumentGrpcError,
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

    const { result, error: jsonRpcError } = await rpcClient.request('broadcast_tx_commit', { tx });

    if (jsonRpcError) {
      const error = new Error();
      Object.assign(error, jsonRpcError);

      throw error;
    }

    const { check_tx: checkTx, deliver_tx: deliverTx } = result;

    if (checkTx.code !== undefined && checkTx.code !== 0) {
      const { error: abciError } = JSON.parse(checkTx.log);

      handleAbciResponseError(
        new AbciResponseError(checkTx.code, abciError),
      );
    }

    if (deliverTx.code !== undefined && deliverTx.code !== 0) {
      const { error: abciError } = JSON.parse(deliverTx.log);

      handleAbciResponseError(
        new AbciResponseError(deliverTx.code, abciError),
      );
    }

    return new BroadcastStateTransitionResponse();
  }

  return broadcastStateTransitionHandler;
}

module.exports = broadcastStateTransitionHandlerFactory;
