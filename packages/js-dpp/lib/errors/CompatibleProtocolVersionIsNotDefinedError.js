class CompatibleProtocolVersionIsNotDefinedError extends Error {
  /**
   * @param {number} currentProtocolVersion
   */
  constructor(currentProtocolVersion) {
    super();

    this.name = this.constructor.name;
    this.message = `Compatible version is not defined for protocol version ${currentProtocolVersion}`;

    this.currentProtocolVersion = currentProtocolVersion;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * @return {number}
   */
  getCurrentProtocolVersion() {
    return this.currentProtocolVersion;
  }
}

module.exports = CompatibleProtocolVersionIsNotDefinedError;
