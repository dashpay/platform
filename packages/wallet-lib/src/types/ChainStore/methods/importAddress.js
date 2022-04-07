const logger = require('../../../logger');
const sortTransactions = require('../../../utils/sortTransactions');

function importAddress(address, reconsiderTransactions = true) {
  logger.silly(`ChainStore - import address ${address}`);

  if (this.state.addresses.has(address.toString())) {
    return;
  }

  this.state.addresses.set(address.toString(), {
    address: address.toString(),
    transactions: [],
    utxos: {},
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
  });

  // TODO: Consider refactoring
  // this code might engage into a cyclic recursive chain of side effects
  // of uncertain complexity
  // (importAddress -> considerTransaction -> importAddress -> ...)
  if (reconsiderTransactions) {
    // We need to consider all previous transactions
    const transactions = [...this.state.transactions.values()];

    const sortedTransactions = sortTransactions(transactions);

    sortedTransactions.forEach((transaction) => {
      this.considerTransaction(transaction.hash);
    });
  }
}

module.exports = importAddress;
