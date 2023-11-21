import validateTenderdashNodeKey from './validators/validateTenderdashNodeKey.js';
import generateTenderdashNodeKey from '../../tenderdash/generateTenderdashNodeKey.js';

/**
 * @param {Object} [options]
 * @param {string} [options.initial]
 * @returns {Object}
 */
export default function createPlatformNodeKeyInput(options = {}) {
  let { initial } = options;
  if (initial === null || initial === undefined) {
    initial = generateTenderdashNodeKey();
  }

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
    initial,
    validate: validateTenderdashNodeKey,
  };
}
