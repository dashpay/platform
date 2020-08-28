class NotSupportedProtocolVersionError extends Error {
  /**
   * @param {number} blockProtocolVersion
   * @param {number} localProtocolVersion
   */
  constructor(blockProtocolVersion, localProtocolVersion) {
    const message = `Block protocol version ${blockProtocolVersion} not supported. Expected to be less or equal to ${localProtocolVersion}.`;
    super(message);
    this.name = this.constructor.name;

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = NotSupportedProtocolVersionError;
