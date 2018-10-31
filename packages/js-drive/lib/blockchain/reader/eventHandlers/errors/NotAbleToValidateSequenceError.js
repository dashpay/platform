class NotAbleToValidateSequenceError extends Error {
  constructor() {
    super();

    this.name = this.constructor.name;
  }
}

module.exports = NotAbleToValidateSequenceError;
