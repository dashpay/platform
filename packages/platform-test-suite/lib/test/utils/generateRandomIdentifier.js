const crypto = require('crypto');
const Dash = require('dash');

const { PlatformProtocol: { Identifier } } = Dash;

/**
 * Generate random identity ID
 *
 * @return {Identifier}
 */
function generateRandomIdentifier() {
  return new Identifier(crypto.randomBytes(32));
}

module.exports = generateRandomIdentifier;
