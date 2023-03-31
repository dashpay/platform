const crypto = require('crypto');
const { default: loadWasmDpp } = require('../../..');

/**
 * Generate random identity ID
 *
 * @return {Identifier}
 */
async function generateRandomIdentifierAsync() {
  const { Identifier } = await loadWasmDpp();
  return new Identifier(crypto.randomBytes(32));
}

module.exports = generateRandomIdentifierAsync;
