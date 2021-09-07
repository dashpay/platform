const AbstractBasicError = require('./AbstractBasicError');

class InvalidIdentifierError extends AbstractBasicError {
  /**
   * @param {string} identifierName
   * @param {string} message
   */
  constructor(identifierName, message) {
    super(`Invalid ${identifierName}: ${message}`);

    this.identifierName = identifierName;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get identifier name
   *
   * @return {string}
   */
  getIdentifierName() {
    return this.identifierName;
  }

  /**
   * Set identifier error
   *
   * @param {Error} error
   */
  setIdentifierError(error) {
    this.identifierError = error;
  }

  /**
   * Get identifier error
   *
   * @return {Error}
   */
  getIdentifierError() {
    return this.identifierError;
  }
}

module.exports = InvalidIdentifierError;
