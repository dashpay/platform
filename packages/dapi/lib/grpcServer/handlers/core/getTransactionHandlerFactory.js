const {
  GetTransactionResponse,
} = require('@dashevo/dapi-grpc');

const { Transaction } = require('@dashevo/dashcore-lib');

const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

/**
 * @param {InsightAPI} insightAPI
 * @returns {getTransactionHandler}
 */
function getTransactionHandlerFactory(insightAPI) {
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

    const serializedTransaction = await insightAPI.getRawTransactionById(id);

    const transaction = new Transaction(serializedTransaction);

    const response = new GetTransactionResponse();
    response.setTransaction(transaction.toBuffer());

    return response;
  }

  return getTransactionHandler;
}

module.exports = getTransactionHandlerFactory;
