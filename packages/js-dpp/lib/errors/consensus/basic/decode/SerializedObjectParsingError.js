const AbstractConsensusError = require('../../AbstractConsensusError');

class SerializedObjectParsingError extends AbstractConsensusError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Parsing of a serialized object failed due to: ${message}`);

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @param {Error} error
   */
  setParsingError(error) {
    this.parsingError = error;
  }

  /**
   * Get parsing error
   *
   * @return {Error}
   */
  getParsingError() {
    return this.parsingError;
  }
}

module.exports = SerializedObjectParsingError;
