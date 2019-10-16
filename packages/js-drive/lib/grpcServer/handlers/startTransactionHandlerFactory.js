const { StartTransactionResponse } = require('@dashevo/drive-grpc');

const {
  server: {
    error: {
      FailedPreconditionGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

/**
 * @param {MongoDBTransaction} stateViewTransaction
 * @returns {startTransactionHandler}
 */
module.exports = function startTransactionHandlerFactory(stateViewTransaction) {
  /**
   * Start new transaction in database
   *
   * @typedef startTransactionHandler
   * @returns {Promise<StartTransactionResponse>}
   */
  async function startTransactionHandler() {
    if (stateViewTransaction.isTransactionStarted) {
      throw new FailedPreconditionGrpcError('Transaction is already started');
    }

    stateViewTransaction.start();

    return new StartTransactionResponse();
  }

  return startTransactionHandler;
};
