const AbstractBasicError = require('../AbstractBasicError');

class DataContractHaveNewIndexWithOldPropertiesError extends AbstractBasicError {
  /**
   * @param {string} documentType
   */
  constructor(documentType) {
    super(`Adding new indices with old properties during Data Contract update is not allowed. Document with type ${documentType} has new index with old properties.`);

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

module.exports = DataContractHaveNewIndexWithOldPropertiesError;
