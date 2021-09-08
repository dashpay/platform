const AbstractConsensusError = require('../AbstractConsensusError');

class UnsupportedProtocolVersionError extends AbstractConsensusError {
  /**
   * @param {number} parsedProtocolVersion
   * @param {number} currentProtocolVersion
   */
  constructor(parsedProtocolVersion, currentProtocolVersion) {
    super(
      `Protocol version ${parsedProtocolVersion} is not supported. Current version is ${currentProtocolVersion}`,
    );

    this.parsedProtocolVersion = parsedProtocolVersion;
    this.currentProtocolVersion = currentProtocolVersion;

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
  getCurrentProtocolVersion() {
    return this.currentProtocolVersion;
  }
}

module.exports = UnsupportedProtocolVersionError;
