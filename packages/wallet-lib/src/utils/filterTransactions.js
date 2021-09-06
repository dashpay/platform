const { uniq, each } = require('lodash');
const { WALLET_TYPES } = require('../CONSTANTS');

const sortByNLockTime = (a, b) => (b.nLockTime - a.nLockTime);
// Will filter out transaction that are not concerning us
// (which can happen in the case of multiple account in store)
function filterTransactions(accountStore, walletType, accountIndex, transactions) {
  /**
   * From transaction's hash, we would need to be able to find the time of such execution.
   * Previously we used 'confirmations' value to estimate the height block where it would
   * be included.
   * This has been removed, and there is no way for us to easily get the block height
   * or hash from a tx.
   * In order to support this feature, it would require us to have the whole raw block set
   * in order to find a tx in a block.
   */
  if (!walletType) throw new Error('Expecting walletType to be provided.');
  if (!accountIndex && accountIndex !== 0) throw new Error('Expecting account index to be provided.');
  if (!transactions) throw new Error('Expecting transactions to be provided');
  if (!accountStore) throw new Error('Expecting accountStore to be provided');
  const filteredTransactions = [];
  const filteredTransactionsId = [];

  const isHDWallet = [WALLET_TYPES.HDWALLET, WALLET_TYPES.HDPUBLIC].includes(walletType);
  const { addresses } = accountStore;
  const { external, internal, misc } = addresses;
  each({ ...external, internal }, (hdAddress) => {
    if (
      hdAddress.path
        && isHDWallet
        && parseInt(hdAddress.path.split('/')[3], 10) === accountIndex
    ) {
      hdAddress.transactions.forEach((txid) => {
        if (!filteredTransactionsId.includes(txid)) {
          filteredTransactionsId.push(txid);
        }
      });
    }
  });

  // Misc addresses can be publicKey/privateKey main addresses if !isHDWallet
  each(misc, (miscAddress) => {
    miscAddress.transactions.forEach((txid) => {
      if (!filteredTransactionsId.includes(txid)) {
        filteredTransactionsId.push(txid);
      }
    });
  });

  uniq(filteredTransactionsId).forEach((transactionId) => {
    const tx = transactions[transactionId];
    filteredTransactions.push(tx);
  });
  return filteredTransactions.sort(sortByNLockTime);
}

module.exports = filterTransactions;
