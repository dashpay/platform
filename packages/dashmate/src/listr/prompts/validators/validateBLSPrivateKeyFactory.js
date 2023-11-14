import { validateHex } from './validateHex.js';
/**
 * @param blsSignatures
 * @returns {validateBLSPrivateKey}
 */

export function validateBLSPrivateKeyFactory(blsSignatures) {
  /**
   * @typedef validateBLSPrivateKey
   * @param {string} value
   * @returns {boolean|string}
   */
  function validateBLSPrivateKey(value) {
    if (value.length === 0) {
      return 'should not be empty';
    }

    if (!validateHex(value)) {
      return 'invalid key format';
    }

    const operatorPrivateKeyBuffer = Buffer.from(value, 'hex');

    let key;
    try {
      key = blsSignatures.PrivateKey.fromBytes(operatorPrivateKeyBuffer, true);
    } catch (e) {
      return 'invalid key';
    } finally {
      if (key) {
        key.delete();
      }
    }

    return true;
  }

  return validateBLSPrivateKey;
}
