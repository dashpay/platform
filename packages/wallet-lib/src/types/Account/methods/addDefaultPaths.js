/**
 * Adds info about default derivation paths to the wallet and chain stores
 */
function addDefaultPaths() {
  const defaultPaths = this.keyChainStore
    .getMasterKeyChain()
    .getIssuedPaths();

  // Add default keychain paths to the account and chain store
  this.addPathsToStore(defaultPaths, true);
}

module.exports = addDefaultPaths;
