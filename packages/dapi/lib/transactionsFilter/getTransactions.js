const {
  Transaction,
} = require('@dashevo/dashcore-lib');

/**
 * @param {CoreRpcClient} coreRpcApi
 * @param {string[]} transactionHashes
 * @return {Promise<Transaction[]>}
 */
async function getTransactions(coreRpcApi, transactionHashes) {
  if (transactionHashes.length === 0) {
    return [];
  }

  const rawTransactions = await coreRpcApi.getRawTransactionMulti(transactionHashes);
  return Object.values(rawTransactions)
    .map((rawTransaction) => new Transaction(rawTransaction));
}

module.exports = getTransactions;
