const Ajv = require('ajv');

class Validator {
  constructor(schema) {
    this.validateArguments = new Ajv().compile(schema);
  }

  validate(args) {
    if (!this.validateArguments(args)) {
      throw new Error(`params${this.validateArguments.errors[0].dataPath} ${this.validateArguments.errors[0].message}`);
    }
  }
}

module.exports = Validator;
