// TODO: consider obtaining addresses only for current account
// instead of the whole keychain
module.exports = function getAddressesToSync() {
  return this.keyChainStore.getKeyChains()
    .map((keychain) => keychain.getWatchedAddresses())
    .reduce((pre, cur) => pre.concat(cur));
};
