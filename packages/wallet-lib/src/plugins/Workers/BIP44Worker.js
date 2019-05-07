const { Worker } = require('../');
const { BIP44_ADDRESS_GAP, WALLET_TYPES } = require('../../CONSTANTS');
const is = require('../../utils/is');

const isContiguousPath = (currPath, prevPath) => {
  if (is.undef(currPath)) return false;
  const splitedCurrPath = currPath.split('/');
  const currIndex = parseInt(splitedCurrPath[5], 10);
  if (is.undef(prevPath)) {
    if (currIndex !== 0) return false;
    return true;
  }
  const splitedPrevPath = prevPath.split('/');
  const prevIndex = parseInt(splitedPrevPath[5], 10);
  if (prevIndex !== currIndex - 1) return false;
  return true;
};
const getMissingIndexes = (paths, fromOrigin = true) => {
  if (!is.arr(paths)) return false;

  let sortedIndexes = [];

  paths.forEach((path) => {
    const splitedPath = path.split('/');
    const index = parseInt(splitedPath[5], 10);
    sortedIndexes.push(index);
  });

  sortedIndexes = sortedIndexes.sort((a, b) => a - b);

  let missingIndex = sortedIndexes.reduce((acc, cur, ind, arr) => {
    const diff = cur - arr[ind - 1];
    if (diff > 1) {
      let i = 1;
      while (i < diff) {
        acc.push(arr[ind - 1] + i);
        i += 1;
      }
    }
    return acc;
  }, []);

  // Will fix missing index before our first known indexes
  if (fromOrigin) {
    if (sortedIndexes[0] > 0) {
      for (let i = sortedIndexes[0] - 1; i >= 0; i -= 1) {
        missingIndex.push(i);
      }
    }
  }

  missingIndex = missingIndex.sort((a, b) => a - b);
  return missingIndex;
};

// TODO : REfacto
class BIP44Worker extends Worker {
  constructor() {
    super({
      name: 'BIP44Worker',
      firstExecutionRequired: true,
      executeOnStart: true,
      dependencies: [
        'storage', 'getAddress', 'walletId', 'accountIndex', 'walletType',
      ],
    });
  }

  ensureEnoughAddress() {
    let generated = 0;
    let unusedAddress = 0;
    const store = this.storage.getStore();
    const addresses = store.wallets[this.walletId].addresses.external;
    let addressesPaths = Object.keys(addresses);
    const { accountIndex, walletType } = this;
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
      .filter(el => parseInt(el.split('/')[3], 10) === accountIndex)
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

      const index = (is.def(lastElem)) ? lastElem.index + 1 : 0;

      const startingIndex = index;
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
  }


  execute() {
    // Following BIP44 Account Discovery section, we will scan the external chain of this account.
    // We do not need to scan the internal as it's linked to external's one
    // So we just seek for 1:1 internal of external.
    this.ensureEnoughAddress();
  }
}

module.exports = BIP44Worker;
