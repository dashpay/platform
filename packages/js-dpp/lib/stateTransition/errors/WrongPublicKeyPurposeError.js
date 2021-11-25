const DPPError = require('../../errors/DPPError');

class WrongPublicKeyPurposeError extends DPPError {
  /**
   *
   * @param {IdentityPublicKey} publicKey
   * @param {number} keyPurposeRequirement
   */
  constructor(publicKey, keyPurposeRequirement) {
    super(`State transition must be signed with a key that has purpose ${keyPurposeRequirement}, but got ${publicKey.getPurpose()}`);

    this.publicKey = publicKey;
    this.keyPurposeRequirement = keyPurposeRequirement;
  }

  /**
   * Get mismatched public key
   *
   * @return {IdentityPublicKey}
   */
  getPublicKey() {
    return this.publicKey;
  }

  /**
   * Get required key purpose
   *
   * @returns {number}
   */
  getKeyPurposeRequirement() {
    return this.keyPurposeRequirement;
  }
}

module.exports = WrongPublicKeyPurposeError;
