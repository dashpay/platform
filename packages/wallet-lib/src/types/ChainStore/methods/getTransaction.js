function getTransaction(transactionHash) {
  return this.state.transactions.get(transactionHash);
}

export default getTransaction;