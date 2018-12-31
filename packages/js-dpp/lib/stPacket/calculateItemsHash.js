const hash = require('../util/hash');
const { encode } = require('../util/serializer');

const calculateItemsHashes = require('./calculateItemsHashes');

/**
 * Calculate hash of ST Packet's items
 *
 * @param {{ objects: Buffer[], contracts: Buffer[] }} items
 * @return {string|null}
 */
function calculateItemsHash(items) {
  const itemsHashes = calculateItemsHashes(items);

  if (itemsHashes.contracts.length === 0
    && itemsHashes.objects.length === 0) {
    return null;
  }

  return hash(encode(itemsHashes));
}

module.exports = calculateItemsHash;
