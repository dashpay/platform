const AbstractSignatureError = require('./AbstractSignatureError');
const Identifier = require('../../../identifier/Identifier');

class IdentityNotFoundError extends AbstractSignatureError {
  /**
   * @param {Buffer} identityId
   */
  constructor(identityId) {
    super(`Identity ${Identifier.from(identityId)} not found`);

    this.identityId = identityId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
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
