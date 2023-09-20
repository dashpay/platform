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

    let rawTransaction;
    const verboseMode = 1;
    try {
      rawTransaction = await coreRPCClient.getRawTransaction(id, verboseMode);
    } catch (e) {
      if (e.code === -5) {
        throw new NotFoundGrpcError('Transaction not found');
      }

      throw e;
    }

    const transaction = new Transaction(rawTransaction.hex);

    const response = new GetTransactionResponse();

    const blockHash = rawTransaction.blockhash ? Buffer.from(rawTransaction.blockhash, 'hex') : Buffer.alloc(0);

    response.setTransaction(transaction.toBuffer());
    response.setBlockHash(blockHash);

    // The height is -1 if transaction is not in the block yet.
    // Do not set it in this case since Protobuf expects uint32
    if (rawTransaction.height >= 0) {
      response.setHeight(rawTransaction.height);
    }

    // Set confirmations might not be present in Core RPC response
    // Double check it just in case
    if (rawTransaction.confirmations >= 0) {
      response.setConfirmations(rawTransaction.confirmations);
    }

    response.setIsInstantLocked(rawTransaction.instantlock_internal);
    response.setIsChainLocked(rawTransaction.chainlock);

    return response;
  }

  return getTransactionHandler;
}

module.exports = getTransactionHandlerFactory;
