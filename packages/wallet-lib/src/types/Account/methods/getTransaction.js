/**
 * Get a transaction from a provided txid
 * @param txid - Transaction Hash
 * @return {Promise<*>}
 */
async function getTransaction(txid = null) {
  const search = await this.storage.searchTransaction(txid);
  if (search.found) {
    return search.result;
  }
  const tx = await this.fetchTransactionInfo(txid);
  try {
    await this.storage.importTransactions(tx);
  } catch (e) {
    console.error(e);
  }
  return tx;
}
module.exports = getTransaction;
