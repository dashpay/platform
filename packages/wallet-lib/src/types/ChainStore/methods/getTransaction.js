function getTransaction(transactionHash) {
  return this.state.transactions.get(transactionHash);
}

module.exports = getTransaction;
