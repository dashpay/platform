const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityPublicKeyDataError extends AbstractBasicError {
  /**
   * @param {number} publicKeyId
   * @param {number} purpose
   * @param {number} securityLevel
   * @param {number[]} allowedSecurityLevels
   */
  constructor(publicKeyId, purpose, securityLevel, allowedSecurityLevels) {
    super(`Invalid identity public key ${publicKeyId} security level: purpose ${purpose} allows only for ${allowedSecurityLevels.join(',')} security levels, but got ${securityLevel}`);

    this.publicKeyId = publicKeyId;
    this.purpose = purpose;
    this.securityLevel = securityLevel;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get identity public key ID
   *
   * @return {number}
   */
  getPublicKeyId() {
    return this.publicKeyId;
  }

  /**
   * Get identity public key purpose
   *
   * @return {number}
   */
  getPublicKeyPurpose() {
    return this.purpose;
  }

  /**
   * Get identity public key security level
   *
   * @return {number}
   */
  getPublicKeySecurityLevel() {
    return this.securityLevel;
  }

  /**
   * @param {Error} error
   */
  setValidationError(error) {
    this.validationError = error;
  }

  /**
   * @return {Error}
   */
  getValidationError() {
    return this.validationError;
  }
}

module.exports = InvalidIdentityPublicKeyDataError;
