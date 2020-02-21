const ConsensusError = require('./ConsensusError');

class SerializedObjectParsingError extends ConsensusError {
  /**
   * @param {Buffer|string} payload
   * @param {Error} parsingError
   */
  constructor(payload, parsingError) {
    super(
      `Parsing of a serialized object failed due to: ${parsingError.message}`,
    );

    this.payload = payload;
    this.parsingError = parsingError;
  }

  /**
   * Get object payload
   *
   * @return {Buffer|string}
   */
  getPayload() {
    return this.payload;
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
