const { Transaction } = require('@dashevo/dashcore-lib');

/**
 * @param {CoreRpcClient} coreAPI
 * @returns {getMemPoolTransactions}
 */
function getMemPoolTransactionsFactory(coreAPI) {
  /**
   * @typedef getMemPoolTransactions
   * @returns {Promise<Transaction[]>}
   */
  async function getMemPoolTransactions() {
    const result = [];
    const memPoolTransactionIds = await coreAPI.getRawMemPool(false);

    for (const txId of memPoolTransactionIds) {
      const rawTransaction = await coreAPI.getRawTransaction(txId);

      const transaction = new Transaction(rawTransaction);
      result.push(transaction);
    }

    return result;
  }

  return getMemPoolTransactions;
}

module.exports = getMemPoolTransactionsFactory;
