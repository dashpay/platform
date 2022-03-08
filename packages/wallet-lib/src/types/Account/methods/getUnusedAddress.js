/**
 * Get an unused address from the store
 * @param {AddressType} [type="external"] - Type of the requested usused address
 * @param {number} [skip=0]
 * @return {AddressObj}
 */
function getUnusedAddress(type = 'external', skip = 0) {
  let unused = {
    address: '',
  };
  const skipped = 0;
  const { walletId } = this;
  const accountIndex = this.index;

  const { addresses } = this.storage.getWalletStore(walletId).getPathState(this.accountPath);

  const chainStore = this.storage.getChainStore(this.network);

  // We sort by type
  const sortedAddresses = {
    external: {},
    internal: {},
  };
  Object
    .keys(addresses)
    .forEach((path) => {
      const splittedPath = path.split('/');
      let pathType = 'external';
      if (splittedPath.length > 1) {
        pathType = (splittedPath[splittedPath.length - 2] === '0') ? 'external' : 'internal';
      }
      sortedAddresses[pathType][path] = addresses[path];
    });

  const keys = Object.keys(sortedAddresses[type]);

  for (let i = 0; i < keys.length; i += 1) {
    const key = keys[i];
    const address = (sortedAddresses[type][key]);
    const addressState = chainStore.getAddress(address);
    if (!addressState || addressState.transactions.length === 0) {
      const keychainData = this.keyChainStore.getMasterKeyChain().getForPath(key);
      unused = {
        address: keychainData.address.toString(),
        path: key,
        index: parseInt(key.split('/').splice(-1)[0], 10),
      };
      break;
    }
  }

  if (skipped < skip) {
    unused = this.getAddress(skipped);
  }
  if (unused.address === '') {
    return this.getAddress(accountIndex, type);
  }
  return unused;
}

module.exports = getUnusedAddress;
