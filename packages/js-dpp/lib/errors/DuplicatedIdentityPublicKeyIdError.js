const ConsensusError = require('./ConsensusError');

class DuplicatedIdentityPublicKeyIdError extends ConsensusError {
  /**
   * @param {RawIdentityPublicKey[]} rawPublicKeys
   */
  constructor(rawPublicKeys) {
    super('Duplicated public key ids found');

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

module.exports = DuplicatedIdentityPublicKeyIdError;
