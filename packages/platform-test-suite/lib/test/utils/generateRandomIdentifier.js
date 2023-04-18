const crypto = require('crypto');
const Dash = require('dash');

const { Platform } = Dash;

/**
 * Generate random identity ID
 *
 * @return {Identifier}
 */
async function generateRandomIdentifier() {
  const { Identifier } = await Platform.initializeDppModule();
  return new Identifier(crypto.randomBytes(32));
}

module.exports = generateRandomIdentifier;
