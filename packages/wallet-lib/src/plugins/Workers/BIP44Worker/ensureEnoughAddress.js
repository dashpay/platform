const { BIP44_ADDRESS_GAP, WALLET_TYPES } = require('../../../CONSTANTS');
const is = require('../../../utils/is');

const getMissingIndexes = require('./utils/getMissingIndexes');
const isContiguousPath = require('./utils/isContiguousPath');

module.exports = function ensureEnoughAddress() {
  let generated = 0;
  let unusedAddress = 0;
  const store = this.storage.getStore();
  const addresses = store.wallets[this.walletId].addresses.external;
  let addressesPaths = Object.keys(addresses);
  const { walletType } = this;
  const accountIndex = this.index;

  let prevPath;

  // Ensure that all our above paths are contiguous
  const missingIndexes = getMissingIndexes(addressesPaths);

  missingIndexes.forEach((index) => {
    this.getAddress(index, 'external');
    if (walletType === WALLET_TYPES.HDWALLET) {
      this.getAddress(index, 'internal');
    }
  });

  const sortByIndex = (a, b) => parseInt(a.split('/')[5], 10) - parseInt(b.split('/')[5], 10);
  addressesPaths = Object
    .keys(store.wallets[this.walletId].addresses.external)
    .filter((el) => parseInt(el.split('/')[3], 10) === accountIndex)
  // sort by index
    .sort(sortByIndex);

  // Scan already generated addresse and count how many are unused
  addressesPaths.forEach((path) => {
    const el = addresses[path];
    if (!el.used && el.transactions.length > 0) {
      el.used = true;
      throw new Error(`Conflicting information ${JSON.stringify(el)}`);
    }
    if (!el.used) unusedAddress += 1;
    if (!isContiguousPath(path, prevPath)) {
      throw new Error('Addresses are expected to be contiguous');
    }
    prevPath = path;
  });

  const addressToGenerate = BIP44_ADDRESS_GAP - unusedAddress;
  if (addressToGenerate > 0) {
    const lastElemPath = addressesPaths[addressesPaths.length - 1];
    const lastElem = addresses[lastElemPath];

    const startingIndex = (is.def(lastElem)) ? lastElem.index + 1 : 0;
    const lastIndex = addressToGenerate + startingIndex;
    if (lastIndex > startingIndex) {
      for (let i = startingIndex; i <= lastIndex; i += 1) {
        this.getAddress(i, 'external');
        generated += 1;
        if (walletType === WALLET_TYPES.HDWALLET) {
          this.getAddress(i, 'internal');
        }
      }
    }
  }

  return generated;
};
