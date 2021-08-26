const DPPError = require('./DPPError');

class CompatibleProtocolVersionIsNotDefinedError extends DPPError {
  /**
   * @param {number} currentProtocolVersion
   */
  constructor(currentProtocolVersion) {
    super(`Compatible version is not defined for protocol version ${currentProtocolVersion}`);

    this.currentProtocolVersion = currentProtocolVersion;
  }

  /**
   * @return {number}
   */
  getCurrentProtocolVersion() {
    return this.currentProtocolVersion;
  }
}

module.exports = CompatibleProtocolVersionIsNotDefinedError;
