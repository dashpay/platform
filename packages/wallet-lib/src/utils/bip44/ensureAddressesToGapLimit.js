const logger = require('../../logger');
const { BIP44_ADDRESS_GAP, WALLET_TYPES } = require('../../CONSTANTS');
const is = require('../is');

const getMissingIndexes = require('./getMissingIndexes');
const isContiguousPath = require('./isContiguousPath');

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

  const externalAddresses = walletStore.addresses.external;
  let externalAddressesPaths = Object.keys(externalAddresses);

  let prevPath;

  // Ensure that all our above external paths are contiguous
  const missingIndexes = getMissingIndexes(externalAddressesPaths);


  // Gets missing addresses and adds them to the storage
  // Please note that getAddress adds new addresses to storage, which it probably shouldn't
  missingIndexes.forEach((index) => {
    getAddress(index, 'external');
    if (walletType === WALLET_TYPES.HDWALLET) {
      getAddress(index, 'internal');
    }
  });

  const sortByIndex = (a, b) => parseInt(a.split('/')[5], 10) - parseInt(b.split('/')[5], 10);
  externalAddressesPaths = Object
    .keys(externalAddresses)
    .filter((el) => parseInt(el.split('/')[3], 10) === accountIndex)
    .sort(sortByIndex);

  let lastUsedIndex = 0;
  let lastGeneratedIndex = 0;

  // Scan already generated addresses and count how many are unused
  externalAddressesPaths.forEach((path) => {
    const address = externalAddresses[path];

    if (!isContiguousPath(path, prevPath)) {
      throw new Error('Addresses are expected to be contiguous');
    }

    if (address.used) {
      lastUsedIndex = address.index;
    }

    lastGeneratedIndex = address.index;
    prevPath = path;
  });

  const gapBetweenLastUsedAndLastGenerated = lastGeneratedIndex - lastUsedIndex;
  const addressToGenerate = BIP44_ADDRESS_GAP - gapBetweenLastUsedAndLastGenerated;

  if (addressToGenerate > 0) {
    const lastElemPath = externalAddressesPaths[externalAddressesPaths.length - 1];
    const lastElem = externalAddresses[lastElemPath];

    const startingIndex = (is.def(lastElem)) ? lastElem.index + 1 : 0;
    const lastIndex = addressToGenerate + startingIndex - 1;

    if (lastIndex > startingIndex) {
      for (let i = startingIndex; i <= lastIndex; i += 1) {
        getAddress(i, 'external');
        generated += 1;
        if (walletType === WALLET_TYPES.HDWALLET) {
          getAddress(i, 'internal');
        }
      }
    }
  }
  logger.silly(`BIP44 - ensured addresses to gap limit - generated: ${generated}`);

  return generated;
}
module.exports = ensureAccountAddressesToGapLimit;
