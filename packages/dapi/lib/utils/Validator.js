const { default: Ajv } = require('ajv/dist/2020');
const ArgumentsValidationError = require('../errors/ArgumentsValidationError');

class Validator {
  constructor(schema) {
    this.validateArguments = new Ajv({
      strictTypes: true,
      strictTuples: true,
      strictRequired: true,
      addUsedSchema: false,
      strict: true,
    }).compile(schema);
  }

  validate(args) {
    if (!this.validateArguments(args)) {
      throw new ArgumentsValidationError(`params${this.validateArguments.errors[0].instancePath} ${this.validateArguments.errors[0].message}`);
    }
  }
}

module.exports = Validator;
