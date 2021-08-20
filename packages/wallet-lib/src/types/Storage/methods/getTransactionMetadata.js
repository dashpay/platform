const { TransactionMetadataNotInStore } = require('../../../errors');

/**
 * Get a specific transaction metadata by it's transaction id
 * @param {string} txid
 * @return {TransactionMetaData}
 */
const getTransactionMetadata = function getTransactionMetadata(txid) {
  const search = this.searchTransactionMetadata(txid);
  if (!search.found) throw new TransactionMetadataNotInStore(txid);
  return search.result;
};

module.exports = getTransactionMetadata;
