const AbstractConsensusError = require('../AbstractConsensusError');

class UnsupportedProtocolVersionError extends AbstractConsensusError {
  /**
   * @param {number} parsedProtocolVersion
   * @param {number} latestVersion
   */
  constructor(parsedProtocolVersion, latestVersion) {
    super(
      `Protocol version ${parsedProtocolVersion} is not supported. Latest supported version is ${latestVersion}`,
    );

    this.parsedProtocolVersion = parsedProtocolVersion;
    this.latestVersion = latestVersion;

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
  getLatestVersion() {
    return this.latestVersion;
  }
}

module.exports = UnsupportedProtocolVersionError;
