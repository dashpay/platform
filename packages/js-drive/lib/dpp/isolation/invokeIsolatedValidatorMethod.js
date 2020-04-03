const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const invokeSyncFunctionFromIsolate = require('./invokeSyncFunctionFromIsolate');
const restoreJsonSchemaErrorConstructor = require('./restoreJsonSchemaErrorConstructor');

/**
 * @param {Context} context
 * @param {string} methodName
 * @param {array} args
 * @param {number} timeout
 * @return {ValidationResult}
 */
function invokeIsolatedValidatorMethod(context, methodName, args, timeout) {
  const result = invokeSyncFunctionFromIsolate(
    context,
    'jsonSchemaValidator',
    methodName,
    args,
    { timeout, arguments: { copy: true }, result: { copy: true } },
  );

  // Restore constructor for JsonSchemaError errors
  result.errors = result.errors.map(restoreJsonSchemaErrorConstructor);

  const validationResult = new ValidationResult();

  return Object.assign(validationResult, result);
}

module.exports = invokeIsolatedValidatorMethod;
