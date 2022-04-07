const sortTransactions = require('../../../utils/sortTransactions');

/**
 * Function goes through all transactions, and ensures address gap
 * having in mind addresses already used by the account.
 */
function createPathsForTransactions() {
  const chainStore = this.storage.getChainStore(this.network);
  const transactions = [...chainStore.getTransactions().values()];

  const sortedTransactions = sortTransactions(transactions);

  sortedTransactions.forEach((transaction, i, self) => {
    // Update the state of UTXO for a given transaction
    const { inputs, outputs } = transaction;

    const affectedAddresses = [];
    [...inputs, ...outputs].forEach((element) => {
      if (element.script) {
        const address = element.script.toAddress(this.network).toString();
        if (chainStore.getAddress(address)) {
          affectedAddresses.push(address);
        }
      }
    });

    // Generate new addresses in case the current set reached it's limit
    // and add them to store
    const paths = this.generateNewPaths(affectedAddresses);

    if (paths && paths.length) {
      const refreshUTXOState = i === self.length - 1;
      this.addPathsToStore(paths, refreshUTXOState);
    }
  });
}

module.exports = createPathsForTransactions;
