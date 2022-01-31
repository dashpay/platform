const { chain } = require('lodash/seq');
const logger = require('../../../logger');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const ensureAddressesToGapLimit = require('../../../utils/bip44/ensureAddressesToGapLimit');

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

  const chainStore = storage.getChainStore(network);
  const accountStore = storage
    .getWalletStore(walletId)
    .getPathState(accountPath);

  const masterKeyChain = keyChainStore.getMasterKeyChain();
  const keyChains = keyChainStore.getKeyChains();
  transactionsWithMayBeMetadata.forEach((transactionWithMetadata) => {
    if (!Array.isArray(transactionWithMetadata)) {
      throw new Error('Expecting transactions to be an array of transaction and metadata elements');
    }
    const [transaction, metadata] = transactionWithMetadata;
    // Affected addresses might not be from our master keychain (account)
    const affectedAddressesData = chainStore.importTransaction(transaction, metadata);
    const affectedAddresses = Object.keys(affectedAddressesData);
    logger.silly(`Account.importTransactions - Import ${transaction.hash} to chainStore. ${affectedAddresses.length} addresses affected.`);

    affectedAddresses.forEach((address) => {
      keyChains.forEach((keyChain) => {
        const issuedPaths = keyChain.markAddressAsUsed(address);
        if (issuedPaths) {
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
  logger.silly(`Account.importTransactions(len: ${transactionsWithMayBeMetadata.length})`);
  return 0;
};
