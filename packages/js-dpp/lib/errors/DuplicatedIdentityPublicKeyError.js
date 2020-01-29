const ConsensusError = require('./ConsensusError');

class DuplicatedIdentityPublicKeyError extends ConsensusError {
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
