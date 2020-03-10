const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  ApplyStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {jaysonClient} rpcClient
 * @param {handleAbciResponse} handleAbciResponse
 * @returns {applyStateTransitionHandler}
 */
function applyStateTransitionHandlerFactory(rpcClient, handleAbciResponse) {
  /**
   * @typedef applyStateTransitionHandler
   * @param {Object} call
   */
  async function applyStateTransitionHandler(call) {
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

    handleAbciResponse(checkTx);

    handleAbciResponse(deliverTx);

    return new ApplyStateTransitionResponse();
  }

  return applyStateTransitionHandler;
}

module.exports = applyStateTransitionHandlerFactory;
