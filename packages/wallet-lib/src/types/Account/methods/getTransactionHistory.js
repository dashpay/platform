const { formatTransactionsForHistory } = require('../../../utils');

/**
 * Get all the transaction history already formated
 * @return {TransactionsHistory}
 */
function getTransactionHistory() {
  const transactions = this.getTransactions();
  return formatTransactionsForHistory(this, transactions);
}

module.exports = getTransactionHistory;
