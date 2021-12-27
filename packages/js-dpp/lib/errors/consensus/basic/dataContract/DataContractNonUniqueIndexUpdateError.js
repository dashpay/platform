const AbstractBasicError = require('../AbstractBasicError');

class DataContractNonUniqueIndexUpdateError extends AbstractBasicError {
  /**
   * @param {string} documentType
   * @param {string} indexName
   */
  constructor(documentType, indexName) {
    super(`Update of non-unique index failed. Document with type ${documentType} has updated index named "${indexName}" out of spec.`);

    this.documentType = documentType;
    this.indexName = indexName;

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

  /**
   * Get updated index name
   *
   * @returns {string}
   */
  getIndexName() {
    return this.indexName;
  }
}

module.exports = DataContractNonUniqueIndexUpdateError;
