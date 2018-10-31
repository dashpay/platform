class WrongSequenceError extends Error {
  constructor() {
    super();

    this.name = this.constructor.name;
  }
}

module.exports = WrongSequenceError;
