const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidSignaturePublicKeySecurityLevelError extends AbstractSignatureError {
  /**
   *
   * @param {number} securityLevel
   */
  constructor(securityLevel) {
    super('Invalid public key security level');

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

module.exports = InvalidSignaturePublicKeySecurityLevelError;
