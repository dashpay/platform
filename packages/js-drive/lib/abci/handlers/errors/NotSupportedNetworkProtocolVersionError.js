const DriveError = require('../../../errors/DriveError');

class NotSupportedNetworkProtocolVersionError extends DriveError {
  /**
   * @param {Long} networkProtocolVersion
   * @param {Long} latestProtocolVersion
   */
  constructor(networkProtocolVersion, latestProtocolVersion) {
    super(`Block protocol version ${networkProtocolVersion} not supported. Expected to be less or equal to ${latestProtocolVersion}.`);

    this.networkProtocolVersion = networkProtocolVersion;
    this.latestProtocolVersion = latestProtocolVersion;
  }

  /**
   * @returns {Long}
   */
  getNetworkProtocolVersion() {
    return this.networkProtocolVersion;
  }

  /**
   * @returns {Long}
   */
  getLatestProtocolVersion() {
    return this.latestProtocolVersion;
  }
}

module.exports = NotSupportedNetworkProtocolVersionError;
