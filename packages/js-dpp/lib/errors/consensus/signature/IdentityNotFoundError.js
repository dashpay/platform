const AbstractSignatureError = require('./AbstractSignatureError');

class IdentityNotFoundError extends AbstractSignatureError {
  /**
   * @param {Buffer} identityId
   */
  constructor(identityId) {
    super('Identity not found');

    this.identityId = identityId;
  }

  /**
   * Get identity id
   *
   * @return {Buffer}
   */
  getIdentityId() {
    return this.identityId;
  }
}

module.exports = IdentityNotFoundError;
