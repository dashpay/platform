const bs58 = require('bs58');
const crypto = require('crypto');

/**
 * Generate DAP Object ID from Blockchain User Id and Slot number (idx)
 *
 * @param blockchainUserId
 * @param slotNumber
 * @returns {string}
 */
function generateDapObjectId(blockchainUserId, slotNumber) {
  const hash = crypto.createHash('sha256');
  hash.update(`${blockchainUserId}${slotNumber}`);
  return bs58.encode(hash.digest());
}

module.exports = generateDapObjectId;
