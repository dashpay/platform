const AbstractBasicError = require('../AbstractBasicError');
const Identifier = require('../../../../identifier/Identifier');

class IncompatibleDataContractSchemaError extends AbstractBasicError {
  /**
   * @param {Buffer} dataContractId
   * @param {string} operation
   * @param {string} fieldPath
   */
  constructor(dataContractId, operation, fieldPath) {
    let message = `Data Contract updated schema is not backward compatible with one defined in Data Contract with id ${Identifier.from(dataContractId)}.`;

    if (operation === 'remove') {
      message = `${message} Field '${fieldPath}' has been removed.`;
    }

    if (operation === 'replace') {
      message = `${message} Field '${fieldPath}' has been changed.`;
    }

    super(message);

    this.dataContractId = dataContractId;
    this.operation = operation;
    this.fieldPath = fieldPath;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get operation
   * @returns {string}
   */
  getOperation() {
    return this.operation;
  }

  /**
   * Get updated field path
   * @returns {string}
   */
  getFieldPath() {
    return this.fieldPath;
  }

  /**
   * Set old data contract schema
   * @param {Object} oldSchema
   */
  setOldSchema(oldSchema) {
    this.oldSchema = oldSchema;
  }

  /**
   * Get old schema
   * @returns {Object}
   */
  getOldSchema() {
    return this.oldSchema;
  }

  /**
   * Set new schema
   * @param {Object} newSchema
   */
  setNewSchema(newSchema) {
    this.newSchema = newSchema;
  }

  /**
   * Get new schema
   * @returns {Object}
   */
  getNewSchema() {
    return this.newSchema;
  }

  /**
   * Set original validation error
   * @param {Error} validationError
   */
  setValidationError(validationError) {
    this.validationError = validationError;
  }

  /**
   * Get orignal validation error
   * @returns {Error}
   */
  getValidationError() {
    return this.validationError;
  }

  /**
   * Get Data Contract ID
   *
   * @return {Buffer}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = IncompatibleDataContractSchemaError;
