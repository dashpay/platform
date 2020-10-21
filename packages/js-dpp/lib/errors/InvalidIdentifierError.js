const ConsensusError = require('./ConsensusError');

class InvalidIdentifierError extends ConsensusError {
  /**
   * @param {string} identifierName
   * @param {IdentifierError} error
   */
  constructor(identifierName, error) {
    super(`Invalid ${identifierName}: ${error.message}`);

    this.identifierName = identifierName;
    this.error = error;
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
   * Get identifier error
   *
   * @return {IdentifierError}
   */
  getIdentifierError() {
    return this.error;
  }
}

module.exports = InvalidIdentifierError;
