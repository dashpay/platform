function validateBLSPrivateKeyFactory(blsSignatures) {
  /**
   * @typedef validateBLSPrivateKey
   * @param {string} value
   * @returns {boolean|string}
   */
  function validateBLSPrivateKey(value) {
    if (value.length === 0) {
      return 'should not be empty';
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

module.exports = validateBLSPrivateKeyFactory;
