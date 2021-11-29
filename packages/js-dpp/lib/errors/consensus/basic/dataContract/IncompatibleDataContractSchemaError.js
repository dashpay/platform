const AbstractBasicError = require('../AbstractBasicError');
const Identifier = require('../../../../identifier/Identifier');

/**
 * Parse schema validation error message
 *
 * @param {Error} schemaValidationError
 *
 * @returns {{ op: string, path: string }[]}
 */
function parseSchemaValidationError(schemaValidationError) {
  const regexp = /change =(.*)$/g;

  const match = schemaValidationError.message.match(regexp);

  return JSON.parse(match[0]);
}

class IncompatibleDataContractSchemaError extends AbstractBasicError {
  /**
   * @param {Buffer} dataContractId
   * @param {Error} schemaValidationError
   */
  constructor(dataContractId, schemaValidationError) {
    const validationErrors = parseSchemaValidationError(schemaValidationError);

    let message = `Data Contract updated schema is not backward compatible with one defined in Data Contract with id ${Identifier.from(dataContractId)}.`;

    const removedField = validationErrors.filter(({ op }) => op === 'remove')[0];
    const replacedField = validationErrors.filter(({ op }) => op === 'replace')[0];

    if (removedField) {
      message = `${message} Field '${removedField.path}' has been removed.`;
    }

    if (replacedField) {
      message = `${message} Field '${replacedField.path}' has been changed.`;
    }

    super(message);

    this.dataContractId = dataContractId;
    this.validationError = schemaValidationError;

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
   * @return {Buffer}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = IncompatibleDataContractSchemaError;
