class MissingChainLockError extends Error {
  constructor() {
    super('ChainLock is required to obtain SML');

    this.name = this.constructor.name;

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = MissingChainLockError;
