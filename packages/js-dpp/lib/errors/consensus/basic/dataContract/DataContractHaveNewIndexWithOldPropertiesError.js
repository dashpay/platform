const AbstractBasicError = require('../AbstractBasicError');

class DataContractHaveNewIndexWithOldPropertiesError extends AbstractBasicError {
  /**
   * @param {string} documentType
   * @param {string} indexName
   */
  constructor(documentType, indexName) {
    super(`Adding new indices with old properties during Data Contract update is not allowed. Document with type ${documentType} has new index with old properties named "${indexName}".`);

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
   * Get index name that have old properties
   *
   * @returns {string}
   */
  getIndexName() {
    return this.indexName;
  }
}

module.exports = DataContractHaveNewIndexWithOldPropertiesError;
