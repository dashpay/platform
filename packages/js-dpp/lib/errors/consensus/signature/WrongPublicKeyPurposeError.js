const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidStateTransitionSignatureError extends AbstractSignatureError {
  /**
   *
   * @param {number} publicKeyPurpose
   * @param {number} keyPurposeRequirement
   */
  constructor(publicKeyPurpose, keyPurposeRequirement) {
    super(`State transition must be signed with a key that has purpose ${keyPurposeRequirement}, but got ${publicKeyPurpose}`);

    this.publicKeyPurpose = publicKeyPurpose;
    this.keyPurposeRequirement = keyPurposeRequirement;
  }

  /**
   * Get mismatched public key
   *
   * @return {number}
   */
  getPublicKeyPurpose() {
    return this.publicKeyPurpose;
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

module.exports = InvalidStateTransitionSignatureError;
