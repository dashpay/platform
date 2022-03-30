const logger = require('../../../logger');

/**
 * Import transactions and always keep a number of unused addresses up to gap
 *
 * @param transactionsWithMayBeMetadata
 * @returns {Promise<{ addressesGenerated: number,  mostRecentHeight: number}>}
 */
module.exports = async function importTransactions(transactionsWithMayBeMetadata) {
  const {
    storage,
    network,
  } = this;

  let addressesGenerated = 0;

  const chainStore = storage.getChainStore(network);

  let mostRecentHeight = -1;
  transactionsWithMayBeMetadata.forEach((transactionWithMetadata) => {
    if (!Array.isArray(transactionWithMetadata)) {
      throw new Error('Expecting transactions to be an array of transaction and metadata elements');
    }
    const [transaction, metadata] = transactionWithMetadata;
    if (metadata && metadata.height > mostRecentHeight) {
      mostRecentHeight = metadata.height;
    }

    const normalizedTransaction = chainStore.importTransaction(transaction, metadata);
    // Affected addresses might not be from our master keychain (account)
    const affectedAddressesData = chainStore.considerTransaction(normalizedTransaction.hash);
    const affectedAddresses = Object.keys(affectedAddressesData);
    logger.silly(`Account.importTransactions - Import ${transaction.hash} to chainStore. ${affectedAddresses.length} addresses affected.`);

    const newPaths = this.generateNewPaths(affectedAddresses);
    addressesGenerated += newPaths.length;
    this.addPathsToStore(newPaths);
  });

  logger.silly(`Account.importTransactions(len: ${transactionsWithMayBeMetadata.length})`);
  return { addressesGenerated, mostRecentHeight };
};
