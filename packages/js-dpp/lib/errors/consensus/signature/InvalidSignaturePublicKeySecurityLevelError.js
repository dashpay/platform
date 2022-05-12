const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidSignaturePublicKeySecurityLevelError extends AbstractSignatureError {
  /**
   *
   * @param {number} publicKeySecurityLevel
   * @param {number} requiredSecurityLevel
   */
  constructor(publicKeySecurityLevel, requiredSecurityLevel) {
    super(`Invalid public key security level: ${publicKeySecurityLevel}. Needs: ${requiredSecurityLevel}.`);

    this.publicKeySecurityLevel = publicKeySecurityLevel;
    this.requiredSecurityLevel = requiredSecurityLevel;
  }

  /**
   * Get mismatched public key
   *
   * @return {number}
   */
  getPublicKeySecurityLevel() {
    return this.publicKeySecurityLevel;
  }

  /**
   * Get required key security level
   *
   * @returns {number}
   */
  getKeySecurityLevelRequirement() {
    return this.requiredSecurityLevel;
  }
}

module.exports = InvalidSignaturePublicKeySecurityLevelError;
