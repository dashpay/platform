const DAPIAddress = require('@dashevo/dapi-client/lib/dapiAddressProvider/DAPIAddress');

/**
 * @param {string} seedsString
 * @returns {RawDAPIAddress[]}
 */
function parseDAPISeedsString(seedsString) {
  return seedsString
    .split(',')
    .map((seed) => new DAPIAddress(seed).toJSON());
}

module.exports = parseDAPISeedsString;
