const { each } = require('lodash');
const {
  categorizeTransactions,
  // calculateTransactionFees,
} = require('../../../utils');

const sortbyTimeDescending = (a, b) => (b.time - a.time);
const sortByHeightDescending = (a, b) => (b.height - a.height);

/**
 * Get all the transaction history already formated
 * @return {TransactionsHistory}
 */
function getTransactionHistory() {
  const transactionHistory = [];

  const {
    walletId,
    walletType,
    index: accountIndex,
    storage,
    network,
  } = this;

  const transactionsWithMetadata = this.getTransactions();
  const walletStore = storage.getWalletStore(walletId);
  const chainStore = storage.getChainStore(network);
  const { blockHeaders } = chainStore.state;

  const categorizedTransactions = categorizeTransactions(
    transactionsWithMetadata,
    walletStore,
    accountIndex,
    walletType,
    network,
  );
  const sortedCategorizedTransactions = categorizedTransactions.sort(sortByHeightDescending);

  each(sortedCategorizedTransactions, (categorizedTransaction) => {
    const {
      transaction,
      from,
      to,
      type,
      isChainLocked,
      isInstantLocked,
    } = categorizedTransaction;
    const blockHash = categorizedTransaction.blockHash !== ''
      ? categorizedTransaction.blockHash
      : null;
    // To get time of block, let's find the blockheader.
    const blockHeader = blockHeaders.get(blockHash);
    // If it's unconfirmed, we won't have a blockHeader nor it's time.
    const time = blockHeader ? blockHeader.time : 9999999999;

    const normalizedTransactionHistory = {
      // Would require knowing the vout of this vin to determinate inputAmount.
      // This information could be fetched, but the necessity vs the cost is questionable.
      //   fees: calculateTransactionFees(categorizedTransaction.transaction),
      from,
      to,
      type,
      time,
      txId: transaction.hash,
      blockHash,
      isChainLocked,
      isInstantLocked,
    };

    transactionHistory.push(normalizedTransactionHistory);
  });

  // Sort by decreasing time.
  return transactionHistory.sort(sortbyTimeDescending);
}

module.exports = getTransactionHistory;
