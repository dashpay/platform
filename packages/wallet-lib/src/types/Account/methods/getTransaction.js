/**
 * Get a transaction from a provided txid
 * @param {transactionId} txid - Transaction Hash
 * @return {Promise<Transaction>}
 */
async function getTransaction(txid = null) {
  const search = await this.storage.searchTransaction(txid);
  if (search.found) {
    return search.result;
  }
  const tx = await this.transport.getTransaction(txid);
  if (this.cacheTx) {
    await this.storage.importTransactions(tx);
    if (this.cacheBlockHeaders) {
      const searchBlockHeader = this.storage.searchBlockHeader(tx.nLockTime);
      if (!searchBlockHeader.found) {
        // Trigger caching of blockheader
        await this.getBlockHeader(tx.nLockTime);
      }
    }
  }
  return tx;
}

module.exports = getTransaction;
