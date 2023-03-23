const generateTenderdashNodeKey = require('../../tenderdash/generateTenderdashNodeKey');
const validateTenderdashNodeKey = require('./validators/validateTenderdashNodeKey');

/**
 * @param {Object} [options]
 * @param {boolean} [options.skipInitial=false]
 * @returns {Object}
 */
function createPlatformNodeKeyInput(options = {}) {
  return {
    type: 'input',
    name: 'platformNodeKey',
    header: `  Dashmate needs to collect details on your Tenderdash node key.

  This key is used to uniquely identify your Dash Platform node. The node key is
  derived from a standard Ed25519 cryptographic key pair, presented in a cached
  format specific to Tenderdash. You can provide a key, or a new key will be
  automatically generated for you.\n`,
    message: 'Enter Ed25519 node key',
    hint: 'Base64 encoded',
    initial: options.skipInitial ? undefined : generateTenderdashNodeKey(),
    validate: validateTenderdashNodeKey,
  };
}

module.exports = createPlatformNodeKeyInput;
