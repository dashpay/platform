class IdentifierError extends Error {
  constructor(message) {
    super();

    this.name = this.constructor.name;
    this.message = message;
  }
}

module.exports = IdentifierError;
