const crypto = require('crypto');

/**
 * Generate entropy
 *
 * @return {Buffer}
 */
function generate() {
  return crypto.randomBytes(32);
}

module.exports = generate;
