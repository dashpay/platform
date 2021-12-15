const AbstractBasicError = require('../AbstractBasicError');

class DataContractHaveNewUniqueIndexError extends AbstractBasicError {
  /**
   * @param {string} documentType
   */
  constructor(documentType) {
    super(`Adding unique indices during Data Contract update is not allowed. Document with type ${documentType} has new unique indices.`);

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

module.exports = DataContractHaveNewUniqueIndexError;
