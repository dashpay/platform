const AbstractBasicError = require('../AbstractBasicError');

class DataContractInvalidIndexDefinitionUpdateError extends AbstractBasicError {
  /**
   * @param {string} documentType
   * @param {string} indexName
   */
  constructor(documentType, indexName) {
    super(`Document with type ${documentType} has badly constructed index "${indexName}". Already indexed properties should be defined in the beginning of index.`);

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

module.exports = DataContractInvalidIndexDefinitionUpdateError;
