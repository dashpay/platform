const AbstractBasicError = require('./AbstractBasicError');

class JsonSchemaError extends AbstractBasicError {
  /**
   * @param {string} message
   * @param {string} keyword
   * @param {string} instancePath
   * @param {string} schemaPath
   * @param {Object} params
   * @param {string} [propertyName]
   */
  constructor(message, keyword, instancePath, schemaPath, params, propertyName) {
    super(message);

    this.keyword = keyword;

    this.instancePath = instancePath;

    this.schemaPath = schemaPath;

    this.params = params;

    this.propertyName = propertyName;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @return {string}
   */
  getKeyword() {
    return this.keyword;
  }

  /**
   * @return {string}
   */
  getInstancePath() {
    return this.instancePath;
  }

  /**
   * @return {string}
   */
  getSchemaPath() {
    return this.schemaPath;
  }

  /**
   * @return {Object}
   */
  getParams() {
    return this.params;
  }

  /**
   * @return {string}
   */
  getPropertyName() {
    return this.propertyName;
  }
}

module.exports = JsonSchemaError;
