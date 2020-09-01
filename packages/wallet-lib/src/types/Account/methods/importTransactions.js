const logger = require('../../../logger');
const { WALLET_TYPES } = require('../../../CONSTANTS');
const ensureAddressesToGapLimit = require('../../../utils/bip44/ensureAddressesToGapLimit');

/**
 * Import transactions and always keep a number of unused addresses up to gap
 *
 * @param transactions
 * @returns {Promise<number>}
 */
module.exports = async function importTransactions(transactions) {
  const {
    walletType,
    walletId,
    index,
    store,
    storage,
    getAddress,
  } = this;

  const localWalletStore = store.wallets[walletId];

  storage.importTransactions(transactions);
  logger.silly(`Account.importTransactions(len: ${transactions.length})`);

  if ([WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(walletType)) {
    // After each imports, we will need to ensure we keep our gap of 20 unused addresses
    return ensureAddressesToGapLimit(
      localWalletStore,
      walletType,
      index,
      getAddress.bind(this),
    );
  }

  return 0;
};
