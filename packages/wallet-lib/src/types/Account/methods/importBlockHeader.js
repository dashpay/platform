const logger = require('../../../logger');
const { WALLET_TYPES } = require('../../../CONSTANTS');
/**
 * Import transactions and always keep a number of unused addresses up to gap
 *
 * @param blockHeader
 * @returns {Promise<number>}
 */
module.exports = async function importBlockHeader(blockHeader) {
  // At this point, the hash of a blockHeader obtained by doing blockHeader.hash,
  // do not seems to be a valid hash.
  // So we will just assume continuous incremental (by one) importing process.

  // We do however have the knowledge of previous block hash by
  // knowing the following blockHeight blockheader's prevHash value
  // const previousHash = blockHeader.prevHash.reverse().toString('hex');
  const {
    walletId, BIP44PATH, index, store, storage, walletType,
  } = this;

  const localWalletStore = store.wallets[walletId];
  const localAccountStore = ([WALLET_TYPES.HDPUBLIC, WALLET_TYPES.HDWALLET].includes(walletType))
    ? localWalletStore.accounts[BIP44PATH.toString()]
    : localWalletStore.accounts[index.toString()];

  localAccountStore.blockHash = blockHeader.id;

  storage.importBlockHeader(blockHeader);
  logger.silly(`Account.importBlockHeader(${blockHeader.id})`);
};
