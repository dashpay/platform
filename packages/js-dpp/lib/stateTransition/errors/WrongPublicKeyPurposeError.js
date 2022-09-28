const DPPError = require('../../errors/DPPError');

class WrongPublicKeyPurposeError extends DPPError {
  /**
   *
   * @param {number} publicKeyPurpose
   * @param {number} keyPurposeRequirement
   */
  constructor(publicKeyPurpose, keyPurposeRequirement) {
    super(`Invalid identity key purpose ${publicKeyPurpose}. This state transition requires ${keyPurposeRequirement}`);

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

module.exports = WrongPublicKeyPurposeError;
