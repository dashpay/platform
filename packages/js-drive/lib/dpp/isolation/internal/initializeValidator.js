const Ajv = require('ajv');

const JsonSchemaError = require('@dashevo/dpp/lib/errors/JsonSchemaError');
const JsonSchemaValidator = require('@dashevo/dpp/lib/validation/JsonSchemaValidator');

// Patch JsonSchemaError in order to pass all properties from isolates
Object.defineProperty(JsonSchemaError.prototype, 'message', {
  get() {
    return JSON.stringify(this);
  },

  set(message) {
    this.originalMessage = message;
  },
});

// Instantiate JsonSchemaValidator
const ajv = new Ajv();

module.exports = new JsonSchemaValidator(ajv);
