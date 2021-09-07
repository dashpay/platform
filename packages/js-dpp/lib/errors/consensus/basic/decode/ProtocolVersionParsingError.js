const AbstractConsensusError = require('../../AbstractConsensusError');

class ProtocolVersionParsingError extends AbstractConsensusError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Can't read protocol version from serialized object: ${message}`);

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

module.exports = ProtocolVersionParsingError;
