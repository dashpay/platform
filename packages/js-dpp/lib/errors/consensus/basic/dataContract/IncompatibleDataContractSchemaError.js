const AbstractBasicError = require('../AbstractBasicError');
const Identifier = require('../../../../identifier/Identifier');

class IncompatibleDataContractSchemaError extends AbstractBasicError {
  /**
   * @param {Object} oldSchema
   * @param {Object} newSchema
   * @param {Error} validationError
   * @param {Buffer|Identifier} dataContractId
   */
  constructor(oldSchema, newSchema, validationError, dataContractId) {
    super(`Data Contract updated schema is not backward compatible with one defined in Data Contract with id ${Identifier.from(dataContractId)}`);

    this.oldSchema = oldSchema;
    this.newSchema = newSchema;
    this.validationError = validationError;
    this.dataContractId = dataContractId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get old schema
   * @returns {Object}
   */
  getOldSchema() {
    return this.oldSchema;
  }

  /**
   * Get new schema
   * @returns {Object}
   */
  getNewSchema() {
    return this.newSchema;
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
