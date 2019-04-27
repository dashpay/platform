const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const calculateItemsHashes = require('./calculateItemsHashes');

/**
 * Calculate hash of ST Packet's items
 *
 * @param {RawSTPacket} rawSTPacket
 * @return {string|null}
 */
function calculateItemsHash(rawSTPacket) {
  const itemsHashes = calculateItemsHashes(rawSTPacket);

  if (itemsHashes.contracts.length === 0
    && itemsHashes.documents.length === 0) {
    return null;
  }

  return hash(encode(itemsHashes)).toString('hex');
}

module.exports = calculateItemsHash;
