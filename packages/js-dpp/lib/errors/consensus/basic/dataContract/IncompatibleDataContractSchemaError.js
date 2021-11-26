const AbstractBasicError = require('../AbstractBasicError');
const Identifier = require('../../../../identifier/Identifier');

class IncompatibleDataContractSchemaError extends AbstractBasicError {
  /**
   * @param {Buffer|Identifier} dataContractId
   */
  constructor(dataContractId) {
    super(`Data Contract updated schema is not backward compatible with one defined in Data Contract with id ${Identifier.from(dataContractId)}`);

    this.dataContractId = dataContractId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
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
   * @return {Buffer|Identifier}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = IncompatibleDataContractSchemaError;
