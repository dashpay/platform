const logger = require('../../../logger');

function importAddress(address) {
  logger.silly(`ChainStore - import address ${address}`);
  if (this.state.addresses.has(address.toString())) throw new Error('Address is already inserted');
  this.state.addresses.set(address.toString(), {
    address: address.toString(),
    transactions: [],
    utxos: {},
    balanceSat: 0,
    unconfirmedBalanceSat: 0,
  });

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

module.exports = importAddress;
