const AbstractConsensusError = require('../../AbstractConsensusError');

class IncompatibleProtocolVersionError extends AbstractConsensusError {
  /**
   * @param {number} parsedProtocolVersion
   * @param {number} minimalProtocolVersion
   */
  constructor(parsedProtocolVersion, minimalProtocolVersion) {
    super(
      `Protocol version ${parsedProtocolVersion} is not supported. Minimal supported protocol version is ${minimalProtocolVersion}`,
    );

    this.parsedProtocolVersion = parsedProtocolVersion;
    this.minimalProtocolVersion = minimalProtocolVersion;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
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
