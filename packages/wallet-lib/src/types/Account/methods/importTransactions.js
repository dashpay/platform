/**
 * Import transactions and always keep a number of unused addresses up to gap
 *
 * @param transactionsWithMayBeMetadata
 * @returns {{ addressesGenerated: number, mostRecentHeight: number }}
 */
module.exports = function importTransactions(transactionsWithMayBeMetadata) {
  const {
    storage,
    network,
  } = this;

  const addressesGenerated = [];

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

    const newPaths = this.generateNewPaths(affectedAddresses);
    newPaths.forEach((path) => {
      addressesGenerated.push(path.address.toString());
    });

    this.addPathsToStore(newPaths);
  });

  return { addressesGenerated, mostRecentHeight };
};
