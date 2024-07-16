const getTransactions = require('./getTransactions');

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
    const memPoolTransactionIds = await coreAPI.getRawMemPool(false);
    return getTransactions(coreAPI, memPoolTransactionIds);
  }

  return getMemPoolTransactions;
}

module.exports = getMemPoolTransactionsFactory;
