const { Transaction } = require('@dashevo/dashcore-lib');

/**
 * @param {RpcClient} coreRpcClient
 * @returns {fetchTransaction}
 */
function fetchTransactionFactory(coreRpcClient) {
  /**
   * @typedef {fetchTransaction}
   * @param {string} id
   * @returns {Transaction}
   */
  async function fetchTransaction(id) {
    let rawTransaction;

    try {
      ({ result: rawTransaction } = await coreRpcClient.getRawTransaction(id, 1));
    } catch (e) {
      // Invalid address or key error
      if (e.code === -5) {
        return null;
      }

      throw e;
    }

    return new Transaction(rawTransaction.hex);
  }

  return fetchTransaction;
}

module.exports = fetchTransactionFactory;
