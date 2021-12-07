const AbstractBasicError = require('../AbstractBasicError');

class DataContractIndicesChangedError extends AbstractBasicError {
  /**
   * @param {string} documentType
   */
  constructor(documentType) {
    super(`Change of indices during Data Contract update is not allowed. Document with type ${documentType} has updated indices.`);

    this.documentType = documentType;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get document type with changed indices
   *
   * @returns {string}
   */
  getDocumentType() {
    return this.documentType;
  }
}

module.exports = DataContractIndicesChangedError;
