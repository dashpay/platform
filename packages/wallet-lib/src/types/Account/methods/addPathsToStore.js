/**
 * Adds info about derivation paths to the wallet and chain stores
 * @param paths - list of new derivation paths
 * @param refreshUTXOState - a flag to trigger side effect in importAddress function
 */
function addPathsToStore(paths, refreshUTXOState = true) {
  const accountStore = this.storage
    .getWalletStore(this.walletId)
    .getPathState(this.accountPath);

  const chainStore = this.storage.getChainStore(this.network);

  paths.forEach((path, i, self) => {
    accountStore.addresses[path.path] = path.address.toString();
    const reconsiderTransactions = refreshUTXOState && i === self.length - 1;
    chainStore.importAddress(path.address.toString(), reconsiderTransactions);
  });
}

module.exports = addPathsToStore;
