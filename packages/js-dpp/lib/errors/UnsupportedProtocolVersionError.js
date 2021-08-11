const ConsensusError = require('./ConsensusError');

class UnsupportedProtocolVersionError extends ConsensusError {
  /**
   * @param {Buffer} payload
   * @param {number} parsedProtocolVersion
   * @param {number} currentProtocolVersion
   */
  constructor(payload, parsedProtocolVersion, currentProtocolVersion) {
    super(
      `Protocol version ${parsedProtocolVersion} is not supported. Current version is ${currentProtocolVersion}`,
    );

    this.payload = payload;
    this.parsedProtocolVersion = parsedProtocolVersion;
    this.currentProtocolVersion = currentProtocolVersion;
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
   * @return {number}
   */
  getParsedProtocolVersion() {
    return this.parsedProtocolVersion;
  }

  /**
   * @return {number}
   */
  getCurrentProtocolVersion() {
    return this.currentProtocolVersion;
  }
}

module.exports = UnsupportedProtocolVersionError;
