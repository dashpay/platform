const AbstractStateError = require('../AbstractStateError');

class InvalidIdentityPublicKeyDisabledError extends AbstractStateError {
  /**
   * @param {number} id
   */
  constructor(id) {
    super('It is impossible to add disabled public key');

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

module.exports = InvalidIdentityPublicKeyDisabledError;
