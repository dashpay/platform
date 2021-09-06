const { each } = require('lodash');
const {
  filterTransactions,
  categorizeTransactions,
  extendTransactionsWithMetadata,
  // calculateTransactionFees,
} = require('../../../utils');

const sortbyTimeDescending = (a, b) => (b.time - a.time);
const sortByHeightDescending = (a, b) => (b.height - a.height);

/**
 * Get all the transaction history already formated
 * @return {Promise<TransactionsHistory>}
 */
async function getTransactionHistory() {
  const transactionHistory = [];

  const {
    walletId,
    walletType,
    index: accountIndex,
    storage,
    network,
  } = this;

  const transactions = this.getTransactions();
  const store = storage.getStore();

  const chainStore = store.chains[network.toString()];
  const { blockHeaders } = chainStore;

  const { wallets: walletStore, transactionsMetadata } = store;

  const accountStore = walletStore[walletId];

  // In store, not all transaction are specific to this account, we filter our transactions.
  const filteredTransactions = filterTransactions(
    accountStore,
    walletType,
    accountIndex,
    transactions,
  );
  const filteredTransactionsWithMetadata = extendTransactionsWithMetadata(
    filteredTransactions,
    transactionsMetadata,
  );

  const categorizedTransactions = categorizeTransactions(
    filteredTransactionsWithMetadata,
    accountStore,
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

    const blockHash = categorizedTransaction.blockHash !== '' ? categorizedTransaction.blockHash : null;

    // To get time of block, let's find the blockheader.
    const blockHeader = blockHeaders[blockHash];

    // If it's unconfirmed, we won't have a blockHeader nor it's time.
    const time = blockHeader ? blockHeader.time : -1;

    const normalizedTransactionHistory = {
      // Would require knowing the vout of this vin to determinate inputAmount.
      // This information could be fetched, but the necessity vs the cost is questionable.
      // fees: calculateTransactionFees(categorizedTransaction.transaction),
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
