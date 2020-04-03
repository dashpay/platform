const JsonSchemaError = require('@dashevo/dpp/lib//errors/JsonSchemaError');

/**
 * @param {Error} error
 * @return {JsonSchemaError|Error}
 */
function restoreJsonSchemaErrorConstructor(error) {
  const [nameAndMessage, ...stackArray] = error.stack.split('\n');
  const [name, serializedProperties] = nameAndMessage.split(': ', 2);

  if (name !== 'JsonSchemaError') {
    return error;
  }

  const errorProperties = JSON.parse(serializedProperties);

  // Restore stack
  errorProperties.stack = `${name}: ${errorProperties.originalMessage}\n${stackArray.join('\n')}`;

  // Restore original message
  errorProperties.message = errorProperties.originalMessage;
  delete errorProperties.originalMessage;

  // Create an empty instance and merge properties
  const jsonSchemaError = new JsonSchemaError(new Error());
  return Object.assign(jsonSchemaError, errorProperties);
}

module.exports = restoreJsonSchemaErrorConstructor;
