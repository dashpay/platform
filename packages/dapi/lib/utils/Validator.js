const Ajv = require('ajv');
const ArgumentsValidationError = require('../errors/ArgumentsValidationError');

class Validator {
  constructor(schema) {
    this.validateArguments = new Ajv().compile(schema);
  }

  validate(args) {
    if (!this.validateArguments(args)) {
      throw new ArgumentsValidationError(`params${this.validateArguments.errors[0].dataPath} ${this.validateArguments.errors[0].message}`);
    }
  }
}

module.exports = Validator;
