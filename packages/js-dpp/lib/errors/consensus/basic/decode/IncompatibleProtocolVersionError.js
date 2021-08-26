const ConsensusError = require('../../ConsensusError');

class IncompatibleProtocolVersionError extends ConsensusError {
  /**
   * @param {Buffer} payload
   * @param {number} parsedProtocolVersion
   * @param {number} minimalProtocolVersion
   */
  constructor(payload, parsedProtocolVersion, minimalProtocolVersion) {
    super(
      `Protocol version ${parsedProtocolVersion} is not supported. Minimal supported protocol version is ${minimalProtocolVersion}`,
    );

    this.payload = payload;
    this.parsedProtocolVersion = parsedProtocolVersion;
    this.minimalProtocolVersion = minimalProtocolVersion;
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
  getMinimalProtocolVersion() {
    return this.minimalProtocolVersion;
  }
}

module.exports = IncompatibleProtocolVersionError;
