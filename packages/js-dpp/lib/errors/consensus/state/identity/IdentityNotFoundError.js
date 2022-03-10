const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class IdentityNotFoundError extends AbstractStateError {
  /**
   * @param {Buffer} identityId
   */
  constructor(identityId) {
    super(`Identity ${Identifier.from(identityId)} doesn't exist`);

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
