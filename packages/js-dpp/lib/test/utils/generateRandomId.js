const crypto = require('crypto');
const bs58 = require('bs58');

/**
 * Generate random identity ID
 *
 * @return {string}
 */
function generateRandomId() {
  const randomBytes = crypto.randomBytes(36);

  const randomHash = crypto.createHash('sha256').update(randomBytes).digest();

  return bs58.encode(randomHash);
}

module.exports = generateRandomId;
