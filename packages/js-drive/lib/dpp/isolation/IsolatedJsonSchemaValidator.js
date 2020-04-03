const invokeIsolatedValidatorMethod = require('./invokeIsolatedValidatorMethod');

class IsolatedJsonSchemaValidator {
  /**
   * @param {Context} isolationContext
   * @param {number} timeout
   */
  constructor(isolationContext, timeout) {
    this.isolationContext = isolationContext;
    this.timeout = timeout;
  }

  /**
   * @param {object} schema
   * @param {object} object
   * @param {array|Object} additionalSchemas
   * @return {ValidationResult}
   */
  validate(schema, object, additionalSchemas = {}) {
    return invokeIsolatedValidatorMethod(
      this.isolationContext,
      'validate',
      [schema, object, additionalSchemas],
      this.timeout,
    );
  }

  /**
   * Validate JSON Schema
   *
   * @param {object} schema
   * @param additionalSchemas
   * @return {ValidationResult}
   */
  validateSchema(schema, additionalSchemas = {}) {
    return invokeIsolatedValidatorMethod(
      this.isolationContext,
      'validateSchema',
      [schema, additionalSchemas],
      this.timeout,
    );
  }
}

module.exports = IsolatedJsonSchemaValidator;
