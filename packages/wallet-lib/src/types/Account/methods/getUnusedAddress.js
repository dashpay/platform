const logger = require('../../../logger');

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
  let skipped = 0;
  const { walletId } = this;
  const accountIndex = this.index;
  const keys = Object.keys(this.store.wallets[walletId].addresses[type])
  // We filter out other potential account
    .filter((el) => parseInt(el.split('/')[3], 10) === accountIndex);

  for (let i = 0; i < keys.length; i += 1) {
    const key = keys[i];
    const el = (this.store.wallets[walletId].addresses[type][key]);

    if (!el || !el.address || el.address === '') {
      logger.warn('getUnusedAddress received an empty one.', el, i, skipped);
    }
    unused = el;
    if (el.used === false) {
      if (skipped < skip) {
        skipped += 1;
      } else {
        break;
      }
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
