const logger = require('../../../logger');

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
  // of uncertain complexity (O(n^2) at minimum)
  // (importAddress -> considerTransaction -> importAddress -> ...)
  if (reconsiderTransactions) {
    // We need to consider all previous transactions
    const transactions = [...this.state.transactions];
    const sortedTransactions = transactions.sort((a, b) => {
      const heightA = a[1].metadata.height;
      const heightB = b[1].metadata.height;
      return heightA - heightB;
    });

    sortedTransactions.forEach(([transactionHash]) => {
      this.considerTransaction(transactionHash);
    });
  }
}

module.exports = importAddress;
