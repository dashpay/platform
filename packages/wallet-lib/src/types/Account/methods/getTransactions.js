/**
 * Get transaction from the store
 * @return {[Transaction]} transactions - All transaction in the store
 */
module.exports = function getTransactions() {
  const store = this.storage.getStore();
  return store.transactions;
};
