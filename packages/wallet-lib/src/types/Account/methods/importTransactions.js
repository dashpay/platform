const logger = require('../../../logger');

/**
 * Import transactions and always keep a number of unused addresses up to gap
 *
 * @param transactions
 * @returns {Promise<number>}
 */
module.exports = async function importTransactions(transactionsWithMayBeMetadata) {
  const {
    storage,
    network,
    walletId,
    accountPath,
    keyChainStore,
  } = this;

  let addressesGenerated = 0;

  const chainStore = storage.getChainStore(network);
  const walletStore = storage.getWalletStore(walletId);
  const accountStore = storage
    .getWalletStore(walletId)
    .getPathState(accountPath);

  const masterKeyChain = keyChainStore.getMasterKeyChain();
  const keyChains = keyChainStore.getKeyChains();

  let mostRecentHeight = -1;
  transactionsWithMayBeMetadata.forEach((transactionWithMetadata) => {
    if (!Array.isArray(transactionWithMetadata)) {
      throw new Error('Expecting transactions to be an array of transaction and metadata elements');
    }
    const [transaction, metadata] = transactionWithMetadata;
    if (metadata && metadata.height > mostRecentHeight) {
      mostRecentHeight = metadata.height;
    }

    // Affected addresses might not be from our master keychain (account)
    const affectedAddressesData = chainStore.importTransaction(transaction, metadata);
    const affectedAddresses = Object.keys(affectedAddressesData);
    logger.silly(`Account.importTransactions - Import ${transaction.hash} to chainStore. ${affectedAddresses.length} addresses affected.`);

    affectedAddresses.forEach((address) => {
      keyChains.forEach((keyChain) => {
        const issuedPaths = keyChain.markAddressAsUsed(address);
        if (issuedPaths) {
          addressesGenerated += issuedPaths.length;
          issuedPaths.forEach((issuedPath) => {
            if (keyChain.keyChainId === masterKeyChain.keyChainId) {
              logger.silly(`Account.importTransactions - newly issued paths ${issuedPath.length}`);
              accountStore.addresses[issuedPath.path] = issuedPath.address.toString();
            }
            chainStore.importAddress(issuedPath.address.toString());
          });
        }
      });
    });
  });

  if (mostRecentHeight !== -1) {
    walletStore.updateLastKnownBlock(mostRecentHeight);
  }

  logger.silly(`Account.importTransactions(len: ${transactionsWithMayBeMetadata.length})`);
  return addressesGenerated;
};
