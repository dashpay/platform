const AbstractBasicError = require('../AbstractBasicError');

class InvalidJsonSchemaRefError extends AbstractBasicError {
  constructor(message) {
    super(`Invalid JSON Schema $ref: ${message}`);

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @param {Error} error
   */
  setRefError(error) {
    this.refError = error;
  }

  /**
   * @returns {Error}
   */
  getRefError() {
    return this.refError;
  }
}

module.exports = InvalidJsonSchemaRefError;
