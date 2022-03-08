module.exports = function getAddressesToSync() {
  return this.keyChainStore.getKeyChains()
    .map((keychain) => keychain.getWatchedAddresses())
    .reduce((pre, cur) => pre.concat(cur));
};
