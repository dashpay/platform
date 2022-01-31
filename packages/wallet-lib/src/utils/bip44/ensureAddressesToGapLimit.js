const logger = require('../../logger');
const { BIP44_ADDRESS_GAP } = require('../../CONSTANTS');
const is = require('../is');

const getMissingIndexes = require('./getMissingIndexes');
const isContiguousPath = require('./isContiguousPath');

const sortByIndex = (a, b) => parseInt(a.split('/')[5], 10) - parseInt(b.split('/')[5], 10);

/**
 * This method ensures there will always be enough local addresses up to gap limit as per BIP44
 * @param {Storage} walletStore
 * @param walletType
 * @param accountIndex
 * @param getAddress
 * @return {number}
 */
function ensureAccountAddressesToGapLimit(walletStore, walletType, accountIndex, getAddress) {
  let generated = 0;

  const { addresses } = walletStore;

  const addressesPaths = {
    external: Object.keys(addresses.external),
    internal: Object.keys(addresses.internal),
  };

  // We need to ensure that all our paths are contiguous, so we first fetch the
  // missing indexes
  const missingIndexes = {
    external: getMissingIndexes(addressesPaths.external),
    internal: getMissingIndexes(addressesPaths.internal),
  };
  // Gets missing addresses and adds them to the storage
  // Please note that getAddress adds new addresses to storage, which it probably shouldn't
  Object.entries(missingIndexes)
    .forEach(([addressesType, indexes]) => {
      indexes.forEach((index) => {
        getAddress(index, addressesType);
      });
    });

  Object.entries(addressesPaths)
    .forEach(([type, paths]) => {
      addressesPaths[type] = paths
        .filter((el) => parseInt(el.split('/')[3], 10) === accountIndex)
        .sort(sortByIndex);
    });

  const lastUsedIndexes = {
    external: -1,
    internal: -1,
  };
  const lastGeneratedIndexes = {
    external: -1,
    internal: -1,
  };

  // Scan already generated addresses and count how many are unused
  Object.entries(addressesPaths)
    .forEach(([type, paths]) => {
      let prevPath;
      paths.forEach((path) => {
        const address = addresses[type][path];
        if (!isContiguousPath(path, prevPath)) {
          throw new Error('Addresses are expected to be contiguous');
        }

        if (address.used) {
          lastUsedIndexes[type] = address.index;
        }

        lastGeneratedIndexes[type] = address.index;
        prevPath = path;
      });
    });

  const gapBetweenLastUsedAndLastGenerated = {
    external: lastGeneratedIndexes.external - lastUsedIndexes.external,
    internal: lastGeneratedIndexes.internal - lastUsedIndexes.internal,
  };
  const addressesToGenerate = {
    external: BIP44_ADDRESS_GAP - gapBetweenLastUsedAndLastGenerated.external,
    internal: BIP44_ADDRESS_GAP - gapBetweenLastUsedAndLastGenerated.internal,
  };

  Object.entries(addressesToGenerate)
    .forEach(([typeToGenerate, numberToGenerate]) => {
      if (numberToGenerate > 0) {
        const pathLength = addressesPaths[typeToGenerate].length;
        const lastElemPath = addressesPaths[typeToGenerate][pathLength - 1];
        const lastElem = addresses[typeToGenerate][lastElemPath];
        const lastExistingIndex = (is.def(lastElem)) ? lastElem.index : -1;
        const lastIndexToGenerate = lastExistingIndex + numberToGenerate;
        if (lastIndexToGenerate > lastExistingIndex) {
          for (
            let index = lastExistingIndex + 1;
            index <= lastIndexToGenerate;
            index += 1) {
            getAddress(index, typeToGenerate);
            generated += 1;
          }
        }
      }
    });

  logger.silly(`BIP44 - ensured addresses to gap limit - generated: ${generated}`);
  return generated;
}

module.exports = ensureAccountAddressesToGapLimit;
