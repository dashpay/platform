const TransactionOkResult = require('./transactionResult/TransactionOkResult');
const TransactionErrorResult = require('./transactionResult/TransactionErrorResult');
const RPCError = require('../../../rpcServer/RPCError');

/**
 * @param {RpcClient} rpcClient
 * @return {getExistingTransactionResult}
 */
function getExistingTransactionResultFactory(rpcClient) {
  /**
   * @typedef {getExistingTransactionResult}
   * @param {string} hashString
   * @return {Promise<TransactionOkResult|TransactionErrorResult>}
   */
  async function getExistingTransactionResult(hashString) {
    const hash = Buffer.from(hashString, 'hex');

    const params = { hash: hash.toString('base64') };

    const { result, error } = await rpcClient.request('tx', params);

    // Handle JSON RPC error
    if (error) {
      throw new RPCError(
        error.code || -32602,
        error.message || 'Internal error',
        error.data,
      );
    }

    const TransactionResultClass = result.tx_result.code === 0
      ? TransactionOkResult
      : TransactionErrorResult;

    return new TransactionResultClass(
      result.tx_result,
      parseInt(result.height, 10),
      Buffer.from(result.tx, 'base64'),
    );
  }

  return getExistingTransactionResult;
}

module.exports = getExistingTransactionResultFactory;
