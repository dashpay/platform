const ValidationError = require('./ValidationError');

class JsonSchemaValidationError extends ValidationError {
  constructor(error) {
    super(error.message);

    Object.assign(this, error);
  }
}

module.exports = JsonSchemaValidationError;
