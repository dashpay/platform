const AbstractStateError = require('../AbstractStateError');

class InvalidIdentityPublicKeyIdError extends AbstractStateError {
  /**
   * @param {number} id
   */
  constructor(id) {
    super(`Identity public key with ID ${id} does not exist`);

    this.id = id;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get ID
   *
   * @return {number}
   */
  getId() {
    return this.id;
  }
}

module.exports = InvalidIdentityPublicKeyIdError;
