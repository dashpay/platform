const DPPError = require('../../errors/DPPError');

class PublicKeySecurityLevelNotMetError extends DPPError {
  /**
   *
   * @param {number} publicKeySecurityLevel
   * @param {number} requiredSecurityLevel
   */
  constructor(publicKeySecurityLevel, requiredSecurityLevel) {
    super(`Invalid key security level ${publicKeySecurityLevel}. This state transition requires at least ${requiredSecurityLevel}`);

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
   * Get minimal required key security level
   *
   * @returns {number}
   */
  getKeySecurityLevelRequirement() {
    return this.requiredSecurityLevel;
  }
}

module.exports = PublicKeySecurityLevelNotMetError;
