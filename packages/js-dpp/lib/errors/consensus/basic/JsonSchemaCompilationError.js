const AbstractBasicError = require('./AbstractBasicError');

class JsonSchemaCompilationError extends AbstractBasicError {
  constructor(message) {
    super(message);

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = JsonSchemaCompilationError;
