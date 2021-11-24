const DPPError = require('../../errors/DPPError');

class PublicKeySecurityLevelNotMetError extends DPPError {
  /**
   *
   * @param {IdentityPublicKey} publicKey
   * @param {number} requiredSecurityLevel
   */
  constructor(publicKey, requiredSecurityLevel) {
    super(`State transition is signed with a key with security level ${publicKey.getSecurityLevel()}, but expected at least ${requiredSecurityLevel}`);

    this.publicKey = publicKey;
    this.requiredSecurityLevel = requiredSecurityLevel;
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
   * Get minimal required key security level
   *
   * @returns {number}
   */
  getSecurityLevel() {
    return this.requiredSecurityLevel;
  }
}

module.exports = PublicKeySecurityLevelNotMetError;
