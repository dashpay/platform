const { Transaction } = require('@dashevo/dashcore-lib');

/**
 * @param {CoreRpcClient} coreAPI
 * @param {testFunction} testTransactionsAgainstFilter
 * @returns {getMemPoolTransactions}
 */
function getMemPoolTransactionsFactory(coreAPI, testTransactionsAgainstFilter) {
  /**
   * @typedef getMemPoolTransactions
   * @param {BloomFilter} filter
   * @returns {Promise<Transaction[]>}
   */
  async function getMemPoolTransactions(filter) {
    const result = [];
    const memPoolTransactionIds = await coreAPI.getRawMemPool(false);

    for (const txId of memPoolTransactionIds) {
      const rawTransaction = await coreAPI.getRawTransaction(txId);
      const transaction = new Transaction(rawTransaction);
      if (testTransactionsAgainstFilter(filter, transaction)) {
        result.push(transaction);
      }
    }

    return result;
  }

  return getMemPoolTransactions;
}

module.exports = getMemPoolTransactionsFactory;
