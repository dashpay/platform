const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidIdentityPublicKeySecurityLevelError extends AbstractSignatureError {
  /**
   * @param {number} securityLevel
   */
  constructor(securityLevel) {
    super(`Invalid identity public key security level ${securityLevel}`);

    this.securityLevel = securityLevel;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get identity public key security level
   *
   * @return {number}
   */
  getSecurityLevel() {
    return this.securityLevel;
  }
}

module.exports = InvalidIdentityPublicKeySecurityLevelError;
