class UnknownActionError extends Error {
  constructor() {
    super();

    this.name = this.constructor.name;
    this.message = 'Can not validate document with unknown action. Please specify in options.';

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }
}

module.exports = UnknownActionError;
