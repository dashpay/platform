const {
  v0: {
    GetTransactionResponse,
  },
} = require('@dashevo/dapi-grpc');

const { Transaction } = require('@dashevo/dashcore-lib');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
      NotFoundGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @returns {getTransactionHandler}
 */
function getTransactionHandlerFactory(coreRPCClient) {
  /**
   * @typedef getTransactionHandler
   * @param {Object} call
   * @returns {Promise<GetTransactionResponse>}
   */
  async function getTransactionHandler(call) {
    const { request } = call;

    const id = request.getId();

    if (!id) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    let serializedTransaction;
    try {
      serializedTransaction = await coreRPCClient.getRawTransaction(id);
    } catch (e) {
      if (e.code === -5) {
        throw new NotFoundGrpcError('Transaction not found');
      }

      throw e;
    }

    const transaction = new Transaction(serializedTransaction);

    const response = new GetTransactionResponse();
    response.setTransaction(transaction.toBuffer());

    return response;
  }

  return getTransactionHandler;
}

module.exports = getTransactionHandlerFactory;
