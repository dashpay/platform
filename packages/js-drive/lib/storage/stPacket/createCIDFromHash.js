const multihashes = require('multihashes');
const CID = require('cids');

const InvalidHashError = require('./errors/InvalidHashError');

/**
 * Create IPFS CID from hash
 *
 * @typedef createCIDFromHash
 * @throws InvalidHashError
 * @param {string} hash
 *
 * @return {CID}
 */
function createCIDFromHash(hash) {
  const buffer = Buffer.from(hash, 'hex');
  const multihash = multihashes.encode(buffer, 'dbl-sha2-256');
  try {
    return new CID(1, 'dag-cbor', multihash);
  } catch (e) {
    throw new InvalidHashError(`could not create CID: ${e.message}`);
  }
}

module.exports = createCIDFromHash;
