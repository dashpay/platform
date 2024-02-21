const crypto = require('crypto');
const { Identifier } = require('../../..');

/**
 * Generate random identity ID
 *
 * @return {Identifier}
 */
function generateRandomIdentifier() {
  return new Identifier(crypto.randomBytes(32));
}

module.exports = generateRandomIdentifier;
