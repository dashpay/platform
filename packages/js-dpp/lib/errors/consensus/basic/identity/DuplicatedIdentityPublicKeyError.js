const AbstractBasicError = require('../AbstractBasicError');

class DuplicatedIdentityPublicKeyError extends AbstractBasicError {
  /**
   * @param {RawIdentityPublicKey[]} rawPublicKeys
   */
  constructor(rawPublicKeys) {
    super('Duplicated public keys found');

    this.rawPublicKeys = rawPublicKeys;
  }

  /**
   * Get public keys
   *
   * @return {RawIdentityPublicKey[]}
   */
  getRawPublicKeys() {
    return this.rawPublicKeys;
  }
}

module.exports = DuplicatedIdentityPublicKeyError;
