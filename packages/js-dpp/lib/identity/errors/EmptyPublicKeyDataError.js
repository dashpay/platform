class EmptyPublicKeyDataError extends Error {
  constructor() {
    super();

    this.message = 'Public key data is not set';

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }
}

module.exports = EmptyPublicKeyDataError;
