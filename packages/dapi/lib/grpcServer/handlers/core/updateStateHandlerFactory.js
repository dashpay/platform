const {
  server: {
    error: {
      InternalGrpcError,
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  UpdateStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {jaysonClient} rpcClient
 * @returns {updateStateHandler}
 */
function updateStateHandlerFactory(rpcClient) {
  /**
   * @param {{code: number, log: string}} response
   */
  function handleResponse(response) {
    if (response.code === undefined || response.code === 0) {
      return;
    }

    const { error: { message, data } } = JSON.parse(response.log);

    switch (response.code) {
      case 2:

        throw new InvalidArgumentGrpcError(message, data);
      case 1:
      default: {
        const e = new Error(message);

        throw new InternalGrpcError(e, data);
      }
    }
  }


  /**
   * @typedef updateStateHandler
   * @param {Object} call
   */
  async function updateStateHandler(call) {
    const { request } = call;
    const stByteArray = request.getData();

    if (!stByteArray) {
      throw new InvalidArgumentGrpcError('State Transition is not specified');
    }

    const tx = Buffer.from(stByteArray).toString('base64');

    let result;
    let error;
    try {
      ({ result, error } = await rpcClient.request('broadcast_tx_commit', { tx }));
    } catch (e) {
      throw new InternalGrpcError(e);
    }

    if (error) {
      throw new InternalGrpcError(error);
    }

    const { check_tx: checkTx, deliver_tx: deliverTx } = result;

    handleResponse(checkTx);

    handleResponse(deliverTx);

    return new UpdateStateTransitionResponse();
  }

  return updateStateHandler;
}

module.exports = updateStateHandlerFactory;
