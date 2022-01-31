/**
 * Get transaction from the store
 * @return {[Transaction]} transactions - All transaction in the store
 */
module.exports = function getTransactions() {
  const chainStore = this.storage.getChainStore(this.network);
  const walletStore = this.storage.getWalletStore(this.walletId);
  const transactions = [];
  const transactionsArray = [];
  const { addresses } = walletStore.getPathState(this.accountPath);

  Object
    .values(addresses)
    .forEach((address) => {
      const addressData = chainStore.getAddress(address);
      if (addressData) {
        const transactionIds = addressData.transactions;
        transactionIds.forEach((transactionId) => {
          const tx = chainStore.getTransaction(transactionId);
          transactions[tx.transaction.hash] = [tx.transaction, tx.metadata];
        });
      }
    });

  Object.entries(transactions)
    .forEach(([, transactionWithMeta]) => {
      transactionsArray.push(transactionWithMeta);
    });

  return transactionsArray;
};
