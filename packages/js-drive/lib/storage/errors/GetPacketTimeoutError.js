class GetPacketTimeoutError extends Error {
  constructor(...params) {
    super(...params);

    this.name = this.constructor.name;

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = GetPacketTimeoutError;
