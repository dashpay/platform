const AbstractStateError = require('../AbstractStateError');

class MissedSecurityLevelIdentityPublicKeyError extends AbstractStateError {
  /**
   *
   * @param {number} securityLevel
   */
  constructor(securityLevel) {
    super(`No public keys left with security level ${securityLevel}`);

    this.securityLevel = securityLevel;
  }

  /**
   *
   * @returns {number}
   */
  getSecurityLevel() {
    return this.securityLevel;
  }
}

module.exports = MissedSecurityLevelIdentityPublicKeyError;
