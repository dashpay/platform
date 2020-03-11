const logger = require('../../../logger');

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
  const tx = await this.transporter.getTransaction(txid);
  if (this.cacheTx) {
    try {
      await this.storage.importTransactions(tx);
      if (this.cacheBlockHeaders) {
        const searchBlockHeader = this.storage.searchBlockHeader(tx.nLockTime);
        if (!searchBlockHeader.found) {
          // Trigger caching of blockheader
          await this.getBlockHeader(tx.nLockTime);
        }
      }
    } catch (e) {
      logger.error('getTransaction', e);
    }
  }
  return tx;
}
module.exports = getTransaction;
