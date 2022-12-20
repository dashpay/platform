const crypto = require('crypto');
const { replace } = require('lodash');
const { default: loadWasmDpp } = require('../../../dist');

/**
 * Generate random identity ID
 *
 * @return {Identifier}
 */
async function generateRandomIdentifierAsync() {
  const { Identifier } = await loadWasmDpp();
  return new Identifier(crypto.randomBytes(32));
}








