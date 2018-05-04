class InvalidParamsError extends Error {
  constructor(...params) {
    super(...params);

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = InvalidParamsError;
