const ConsensusError = require('./ConsensusError');

class ProtocolVersionParsingError extends ConsensusError {
  /**
   * @param {Buffer} payload
   * @param {Error} parsingError
   */
  constructor(payload, parsingError) {
    super(
      `Can't read protocol version from serialized object: ${parsingError.message}`,
    );

    this.payload = payload;
    this.parsingError = parsingError;
  }

  /**
   * Get object payload
   *
   * @return {Buffer}
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

module.exports = ProtocolVersionParsingError;
