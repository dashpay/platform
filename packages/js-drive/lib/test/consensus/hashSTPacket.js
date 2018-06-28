const cbor = require('cbor');
const multihashingAsync = require('multihashing-async');
const multihashes = require('multihashes');
const util = require('util');

const multihashing = util.promisify(multihashingAsync);

/**
 * Encode State Transisition packet
 *
 * @param packet
 * @returns {Promise<String>}
 */
async function hashSTPacket(packet) {
  const serializedPacket = cbor.encodeCanonical(packet);
  const multihash = await multihashing(serializedPacket, 'sha2-256');
  const decoded = multihashes.decode(multihash);
  return decoded.digest.toString('hex');
}

module.exports = hashSTPacket;
