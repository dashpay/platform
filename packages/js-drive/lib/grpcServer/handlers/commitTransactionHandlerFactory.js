const { CommitTransactionResponse } = require('@dashevo/drive-grpc');

const {
  server: {
    error: {
      InternalGrpcError,
      FailedPreconditionGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

/**
 * @param {MongoDBTransaction} stateViewTransaction
 * @param {createContractDatabase} createContractDatabase
 * @param {removeContractDatabase} removeContractDatabase
 * @param {BlockExecutionState} blockExecutionState
 * @returns {commitTransactionHandler}
 */
module.exports = function commitTransactionHandlerFactory(
  stateViewTransaction,
  createContractDatabase,
  removeContractDatabase,
  blockExecutionState,
) {
  /**
   * Commit transaction, that was created before and create collections for
   * documents inside of received contracts
   *
   * @typedef commitTransactionHandler
   * @returns {Promise<CommitTransactionResponse>}
   */
  async function commitTransactionHandler() {
    if (!stateViewTransaction.isTransactionStarted) {
      throw new FailedPreconditionGrpcError('Transaction is not started');
    }

    const contracts = blockExecutionState.getContracts();
    const createdContracts = [];

    try {
      for (const contract of contracts) {
        await createContractDatabase(contract);
        createdContracts.push(contract);
      }

      await stateViewTransaction.commit();
    } catch (error) {
      if (stateViewTransaction.isTransactionStarted) {
        await stateViewTransaction.abort();
      }

      for (const contract of createdContracts) {
        await removeContractDatabase(contract);
      }

      throw new InternalGrpcError(error);
    }

    blockExecutionState.clearContracts();

    return new CommitTransactionResponse();
  }

  return commitTransactionHandler;
};
