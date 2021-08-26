const AbstractBasicError = require('./AbstractBasicError');

class JsonSchemaError extends AbstractBasicError {
  constructor(error) {
    super(error.message);

    Object.assign(this, error);
  }
}

module.exports = JsonSchemaError;
